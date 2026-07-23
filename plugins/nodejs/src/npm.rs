//! Shared `npm` command helpers: cwd-seeded builder, path resolution against the wasm
//! preopen, package.json pre-flight, lockfile detection, wiped output dirs, the uniform
//! exit phrase, and the optional `npm version` step shared by `build`/`pack`.

use moonlit_plugin_sdk::prelude::*; // Context, MiddlewareResult, LineHandler, Deserialize, …
use moonlit_plugin_sdk::process::Command;
use std::path::PathBuf;

/// An `npm` command pre-seeded with `directory` (resolved against the working dir) as cwd.
pub fn npm<'a>(ctx: &Context<'a>, directory: &str) -> Command<'a> {
    // The spawn cwd is a HOST path — the engine passes it straight to the OS process's
    // `current_dir` — so it must always include the working dir. This differs from
    // `resolve`, which is preopen-relative under wasm for the plugin's own filesystem
    // access; using `resolve` here would drop the working dir on wasm and spawn npm in
    // the engine's cwd instead of the project's.
    let cwd = std::path::Path::new(ctx.working_dir())
        .join(directory)
        .to_string_lossy()
        .into_owned();
    ctx.command("npm").cwd(cwd)
}

/// Resolve a path against the working dir. Under wasm the preopen IS the working dir
/// (`.`), so a relative path is correct; native tests join the host dir.
#[cfg(target_arch = "wasm32")]
pub fn resolve(_working_dir: &str, p: &str) -> PathBuf {
    PathBuf::from(p)
}
#[cfg(not(target_arch = "wasm32"))]
pub fn resolve(working_dir: &str, p: &str) -> PathBuf {
    std::path::Path::new(working_dir).join(p)
}

/// The uniform non-zero-exit failure phrase.
pub fn exit_phrase(code: i32) -> String {
    format!("Npm command failed with exit code {code}")
}

/// Confirm `directory` contains a `package.json`; on absence return the frozen message
/// naming the resolved directory.
pub fn require_package_json(working_dir: &str, directory: &str) -> Result<(), String> {
    let dir = resolve(working_dir, directory);
    if dir.join("package.json").is_file() {
        Ok(())
    } else {
        Err(format!(
            "package.json not found in directory: {}",
            dir.display()
        ))
    }
}

/// Whether `directory` holds an npm lockfile (`package-lock.json` / `npm-shrinkwrap.json`).
pub fn has_lockfile(working_dir: &str, directory: &str) -> bool {
    let dir = resolve(working_dir, directory);
    dir.join("package-lock.json").is_file() || dir.join("npm-shrinkwrap.json").is_file()
}

/// Create a fresh (wiped) directory at `rel` under the working dir; returns the resolved
/// path. Wiping gives clock-free per-run isolation.
pub fn prepare_output_dir(working_dir: &str, rel: &str) -> std::io::Result<PathBuf> {
    let dir = resolve(working_dir, rel);
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Optional `npm version <v> --no-git-tag-version --allow-same-version` step (used by
/// `build`/`pack`). Returns `Some(failure)` to early-return; `None` on success or when
/// `version` is absent/blank.
pub fn maybe_set_version(
    ctx: &Context,
    directory: &str,
    version: &Option<String>,
) -> Option<MiddlewareResult> {
    let v = version.as_deref().filter(|s| !s.trim().is_empty())?;
    let args = vec![
        "version".to_string(),
        v.to_string(),
        "--no-git-tag-version".to_string(),
        "--allow-same-version".to_string(),
    ];
    match npm(ctx, directory)
        .args(args)
        .stream(LineHandler::severity())
    {
        Ok(o) if o.success() => None,
        Ok(o) => Some(MiddlewareResult::failure(format!(
            "Failed to set version: {}",
            exit_phrase(o.exit_code)
        ))),
        Err(e) => Some(MiddlewareResult::failure(format!(
            "Failed to set version: {e}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_plugin_sdk::testing::MockHost;

    #[test]
    fn npm_builder_sets_program_and_cwd() {
        let host = MockHost::new().with_process_result(0, vec![]);
        let ctx = Context::new(&host, "/wd".into(), "s".into());
        let _ = npm(&ctx, ".").arg("ci").run();
        let cmds = host.recorded_commands();
        assert_eq!(cmds[0].program, "npm");
        assert_eq!(cmds[0].args, vec!["ci".to_string()]);
        // cwd is the resolved "." under the working dir (native: "/wd/.").
        assert!(cmds[0].cwd.as_deref().unwrap().starts_with("/wd"));
    }

    #[test]
    fn npm_cwd_prepends_working_dir_for_subdirectory() {
        // The spawn cwd is a host path: it must include the working dir, not just the
        // relative `directory` — regression guard for the wasm cwd bug.
        let host = MockHost::new().with_process_result(0, vec![]);
        let ctx = Context::new(&host, "/wd".into(), "s".into());
        let _ = npm(&ctx, "packages/app").arg("ci").run();
        assert_eq!(
            host.recorded_commands()[0].cwd.as_deref(),
            Some("/wd/packages/app")
        );
    }

    #[test]
    fn exit_phrase_formats_code() {
        assert_eq!(exit_phrase(7), "Npm command failed with exit code 7");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn require_package_json_ok_and_missing() {
        let d = tempfile::tempdir().unwrap();
        let wd = d.path().to_str().unwrap();
        match require_package_json(wd, ".") {
            Ok(()) => panic!("expected missing package.json"),
            Err(m) => assert!(m.starts_with("package.json not found in directory:")),
        }
        std::fs::write(d.path().join("package.json"), b"{}").unwrap();
        assert!(require_package_json(wd, ".").is_ok());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn has_lockfile_detects_both_npm_lockfiles() {
        let d = tempfile::tempdir().unwrap();
        let wd = d.path().to_str().unwrap();
        assert!(!has_lockfile(wd, "."));
        std::fs::write(d.path().join("package-lock.json"), b"{}").unwrap();
        assert!(has_lockfile(wd, "."));
        std::fs::remove_file(d.path().join("package-lock.json")).unwrap();
        std::fs::write(d.path().join("npm-shrinkwrap.json"), b"{}").unwrap();
        assert!(has_lockfile(wd, "."));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn prepare_output_dir_wipes_existing_contents() {
        let d = tempfile::tempdir().unwrap();
        let wd = d.path().to_str().unwrap();
        let first = prepare_output_dir(wd, ".moonlit/npm-pack").unwrap();
        std::fs::write(first.join("stale.tgz"), b"x").unwrap();
        let second = prepare_output_dir(wd, ".moonlit/npm-pack").unwrap();
        assert_eq!(first, second);
        assert!(
            !second.join("stale.tgz").exists(),
            "must wipe prior contents"
        );
    }

    #[test]
    fn maybe_set_version_noop_when_blank() {
        let host = MockHost::new();
        let ctx = Context::new(&host, "/wd".into(), "build".into());
        assert!(maybe_set_version(&ctx, ".", &None).is_none());
        assert!(maybe_set_version(&ctx, ".", &Some("  ".to_string())).is_none());
        assert!(host.recorded_commands().is_empty());
    }

    #[test]
    fn maybe_set_version_runs_expected_argv() {
        let host = MockHost::new().with_process_result(0, vec![]);
        let ctx = Context::new(&host, "/wd".into(), "build".into());
        assert!(maybe_set_version(&ctx, ".", &Some("1.2.3".to_string())).is_none());
        let cmds = host.recorded_commands();
        assert_eq!(
            cmds[0].args,
            vec![
                "version",
                "1.2.3",
                "--no-git-tag-version",
                "--allow-same-version",
            ]
        );
    }

    #[test]
    fn maybe_set_version_failure_maps_message() {
        let host = MockHost::new().with_process_result(1, vec![]);
        let ctx = Context::new(&host, "/wd".into(), "build".into());
        let f = maybe_set_version(&ctx, ".", &Some("1.2.3".to_string()))
            .expect("expected failure")
            .into_wit();
        assert_eq!(
            f.error_message.as_deref(),
            Some("Failed to set version: Npm command failed with exit code 1")
        );
    }
}
