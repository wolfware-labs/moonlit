//! `npm pack --json` — optionally bump the version, produce a tarball into a wiped
//! destination, and emit its working-dir-relative `packagePath`.

use crate::npm::{
    exit_phrase, maybe_set_version, npm, prepare_output_dir, require_package_json, resolve,
};
use moonlit_sdk::prelude::*;
use moonlit_sdk::process::LineHandler;

/// The first tarball filename from `npm pack --json` stdout (a JSON array of results).
/// `None` on malformed JSON or an empty array.
fn parse_pack_filename(stdout: &str) -> Option<String> {
    let arr: serde_json::Value = serde_json::from_str(stdout).ok()?;
    arr.as_array()?
        .first()?
        .get("filename")?
        .as_str()
        .map(str::to_string)
}

/// Working-dir-relative path to the pack destination given the npm cwd `directory` and a
/// cwd-relative `dest`. `.`/empty directory leaves `dest` unchanged.
fn dest_wd_rel(directory: &str, dest: &str) -> String {
    let d = directory.trim_end_matches('/');
    if d.is_empty() || d == "." {
        dest.to_string()
    } else {
        format!("{d}/{dest}")
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PackConfig {
    pub directory: String,
    pub version: Option<String>,
    pub destination: Option<String>,
}

impl Default for PackConfig {
    fn default() -> Self {
        Self {
            directory: ".".to_string(),
            version: None,
            destination: None,
        }
    }
}

#[derive(Default)]
pub struct Pack;

impl Middleware for Pack {
    const NAME: &'static str = "pack";
    const DESCRIPTION: &'static str = "pack the package into a .tgz tarball";
    type Config = PackConfig;

    fn execute(&self, ctx: &Context, cfg: PackConfig) -> MiddlewareResult {
        if let Err(msg) = require_package_json(ctx.working_dir(), &cfg.directory) {
            return MiddlewareResult::failure(msg);
        }
        if let Some(fail) = maybe_set_version(ctx, &cfg.directory, &cfg.version) {
            return fail;
        }
        // `npm_dest` is relative to npm's cwd (the directory); `wd_rel` is the same path
        // made working-dir-relative for readback + the emitted packagePath.
        let user_dest = cfg.destination.as_deref().filter(|s| !s.trim().is_empty());
        let npm_dest = user_dest
            .map(str::to_string)
            .unwrap_or_else(|| ".moonlit/npm-pack".to_string());
        let wd_rel = dest_wd_rel(&cfg.directory, &npm_dest);
        // Default dir is wiped per run; a user-provided destination is create-if-missing
        // only (never delete user files).
        let prep = if user_dest.is_some() {
            std::fs::create_dir_all(resolve(ctx.working_dir(), &wd_rel)).map(|_| ())
        } else {
            prepare_output_dir(ctx.working_dir(), &wd_rel).map(|_| ())
        };
        if let Err(e) = prep {
            return MiddlewareResult::failure(format!("Failed to prepare pack destination: {e}"));
        }

        let args = vec![
            "pack".to_string(),
            "--pack-destination".to_string(),
            npm_dest.clone(),
            "--json".to_string(),
        ];
        let out = match npm(ctx, &cfg.directory)
            .args(args)
            .stream(LineHandler::severity())
        {
            Ok(o) if o.success() => o,
            Ok(o) => {
                return MiddlewareResult::failure(format!(
                    "Failed to pack project: {}",
                    exit_phrase(o.exit_code)
                ))
            }
            Err(e) => return MiddlewareResult::failure(format!("Failed to pack project: {e}")),
        };

        match parse_pack_filename(&out.stdout()) {
            Some(filename) => {
                let package_path = format!("{wd_rel}/{filename}");
                MiddlewareResult::success_with(|o| o.set("packagePath", package_path))
            }
            None => MiddlewareResult::failure("No package tarball was created."),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_sdk::testing::{run, MockHost};

    fn proj_dir() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("package.json"), b"{}").unwrap();
        d
    }
    fn ctx<'a>(host: &'a MockHost, dir: &std::path::Path) -> Context<'a> {
        Context::new(host, dir.to_str().unwrap().into(), "pack".into())
    }

    // --- pure helpers ---
    #[test]
    fn parse_pack_filename_single_object() {
        let s = r#"[{"id":"pkg@1.0.0","name":"pkg","filename":"pkg-1.0.0.tgz"}]"#;
        assert_eq!(parse_pack_filename(s).as_deref(), Some("pkg-1.0.0.tgz"));
    }
    #[test]
    fn parse_pack_filename_empty_array_is_none() {
        assert!(parse_pack_filename("[]").is_none());
    }
    #[test]
    fn parse_pack_filename_malformed_is_none() {
        assert!(parse_pack_filename("not json").is_none());
    }
    #[test]
    fn parse_pack_filename_takes_first_of_many() {
        let s = r#"[{"filename":"a.tgz"},{"filename":"b.tgz"}]"#;
        assert_eq!(parse_pack_filename(s).as_deref(), Some("a.tgz"));
    }
    #[test]
    fn dest_wd_rel_dot_directory_unchanged() {
        assert_eq!(dest_wd_rel(".", ".moonlit/npm-pack"), ".moonlit/npm-pack");
        assert_eq!(dest_wd_rel("", ".moonlit/npm-pack"), ".moonlit/npm-pack");
    }
    #[test]
    fn dest_wd_rel_subdir_prefixed() {
        assert_eq!(
            dest_wd_rel("packages/app", ".moonlit/npm-pack"),
            "packages/app/.moonlit/npm-pack"
        );
        assert_eq!(dest_wd_rel("packages/app/", "d"), "packages/app/d");
    }

    // --- Pack middleware ---
    // MockHost writes no tarball, so a successful spawn with empty stdout resolves to the
    // "no tarball" path; argv tests assert the recorded command and ignore the result.
    #[test]
    fn pack_builds_default_argv() {
        let d = proj_dir();
        let host = MockHost::new().with_process_result(0, vec![]);
        let _ = run(&Pack, &ctx(&host, d.path()), PackConfig::default());
        assert_eq!(
            host.recorded_commands()[0].args,
            vec!["pack", "--pack-destination", ".moonlit/npm-pack", "--json"]
        );
    }

    #[test]
    fn pack_version_step_precedes_pack() {
        let d = proj_dir();
        let host = MockHost::new()
            .with_process_result(0, vec![])
            .with_process_result(0, vec![]);
        let cfg = PackConfig {
            version: Some("1.4.0".into()),
            ..Default::default()
        };
        let _ = run(&Pack, &ctx(&host, d.path()), cfg);
        let cmds = host.recorded_commands();
        assert_eq!(cmds[0].args[0], "version");
        assert_eq!(cmds[1].args[0], "pack");
    }

    #[test]
    fn pack_custom_destination_used_in_argv() {
        let d = proj_dir();
        let host = MockHost::new().with_process_result(0, vec![]);
        let cfg = PackConfig {
            destination: Some("out/tarballs".into()),
            ..Default::default()
        };
        let _ = run(&Pack, &ctx(&host, d.path()), cfg);
        assert_eq!(
            host.recorded_commands()[0].args,
            vec!["pack", "--pack-destination", "out/tarballs", "--json"]
        );
    }

    #[test]
    fn pack_emits_working_dir_relative_package_path() {
        use moonlit_sdk::process::{OutputChunk, StdioStream};
        let d = proj_dir();
        // A stdout chunk carrying a valid `npm pack --json` array.
        let json_chunk = OutputChunk {
            stream: StdioStream::Stdout,
            text: r#"[{"filename":"pkg-1.0.0.tgz"}]"#.to_string(),
        };
        let host = MockHost::new().with_process_result(0, vec![json_chunk]);
        let w = run(&Pack, &ctx(&host, d.path()), PackConfig::default()).into_wit();
        assert!(w.successful);
        let pp = w
            .output
            .iter()
            .find(|(k, _)| k == "packagePath")
            .map(|(_, v)| v.clone())
            .unwrap();
        // Output values are JSON text, so a string is quoted.
        assert_eq!(pp, "\".moonlit/npm-pack/pkg-1.0.0.tgz\"");
    }

    #[test]
    fn pack_no_tarball_fails() {
        let d = proj_dir();
        let host = MockHost::new().with_process_result(0, vec![]); // empty stdout
        let w = run(&Pack, &ctx(&host, d.path()), PackConfig::default()).into_wit();
        assert_eq!(
            w.error_message.as_deref(),
            Some("No package tarball was created.")
        );
    }

    #[test]
    fn pack_missing_package_json_fails_before_spawn() {
        let d = tempfile::tempdir().unwrap();
        let host = MockHost::new();
        let w = run(&Pack, &ctx(&host, d.path()), PackConfig::default()).into_wit();
        assert!(w
            .error_message
            .unwrap()
            .starts_with("package.json not found in directory:"));
        assert!(host.recorded_commands().is_empty());
    }

    #[test]
    fn pack_user_destination_is_not_wiped() {
        use moonlit_sdk::process::{OutputChunk, StdioStream};
        let d = proj_dir();
        // Seed a user file in the destination; pack must NOT delete it (create-only, no wipe).
        let dest = d.path().join("out/tarballs");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("keep.txt"), b"important").unwrap();
        let json_chunk = OutputChunk {
            stream: StdioStream::Stdout,
            text: r#"[{"filename":"pkg-1.0.0.tgz"}]"#.to_string(),
        };
        let host = MockHost::new().with_process_result(0, vec![json_chunk]);
        let cfg = PackConfig {
            destination: Some("out/tarballs".into()),
            ..Default::default()
        };
        let _ = run(&Pack, &ctx(&host, d.path()), cfg);
        assert!(
            dest.join("keep.txt").exists(),
            "user destination must not be wiped"
        );
    }

    #[test]
    fn pack_blank_destination_falls_back_to_default() {
        let d = proj_dir();
        let host = MockHost::new().with_process_result(0, vec![]);
        let cfg = PackConfig {
            destination: Some("".into()),
            ..Default::default()
        };
        let _ = run(&Pack, &ctx(&host, d.path()), cfg);
        assert_eq!(
            host.recorded_commands()[0].args,
            vec!["pack", "--pack-destination", ".moonlit/npm-pack", "--json"]
        );
    }
}
