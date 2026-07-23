//! `npm publish <tgz>` — publish a tarball via a scoped `--userconfig .npmrc` (token stays
//! off the argv), classifying auth and version-conflict failures.

use crate::config::NodeConfig;
use crate::npm::{exit_phrase, npm, prepare_output_dir, resolve};
use moonlit_plugin_sdk::prelude::*;
use moonlit_plugin_sdk::process::LineHandler;
use std::path::Path;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PushConfig {
    pub package: String,
    pub registry: Option<String>,
    pub token: Option<String>,
    pub tag: String,
    pub access: Option<String>,
}

impl Default for PushConfig {
    fn default() -> Self {
        Self {
            package: String::new(),
            registry: None,
            token: None,
            tag: "latest".to_string(),
            access: None,
        }
    }
}

/// The scoped `.npmrc` auth line for `registry`: strip the scheme, keep host+path, one
/// trailing slash, then `:_authToken=<token>`.
fn npmrc_line(registry: &str, token: &str) -> String {
    let stripped = registry
        .strip_prefix("https://")
        .or_else(|| registry.strip_prefix("http://"))
        .unwrap_or(registry);
    format!("//{}/:_authToken={token}", stripped.trim_end_matches('/'))
}

fn non_blank(s: &str) -> bool {
    !s.trim().is_empty()
}

/// Write the scoped `.npmrc` (`dir/.npmrc`). On Unix the file is created owner-only
/// (`0o600`) *before* the token is written, so the credential is never briefly
/// world-readable. On the wasm target — WASI preview 2 has no Unix mode bits, so the
/// file's permissions are the host's responsibility — this degrades to a plain write.
fn write_npmrc(dir: &Path, contents: &str) -> std::io::Result<()> {
    let path = dir.join(".npmrc");
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&path)?;
        f.write_all(contents.as_bytes())
    }
    #[cfg(not(unix))]
    {
        std::fs::write(&path, contents)
    }
}

#[derive(Default)]
pub struct Push;

impl Middleware for Push {
    const NAME: &'static str = "push";
    const DESCRIPTION: &'static str = "publish a .tgz tarball to an npm registry";
    type Config = PushConfig;

    fn execute(&self, ctx: &Context, cfg: PushConfig) -> MiddlewareResult {
        let pkg_path = resolve(ctx.working_dir(), &cfg.package);
        if !pkg_path.is_file() {
            return MiddlewareResult::failure(format!(
                "Package tarball not found at path: {}",
                pkg_path.display()
            ));
        }

        let plugin = ctx.plugin_config::<NodeConfig>();
        let registry = cfg
            .registry
            .as_deref()
            .filter(|s| non_blank(s))
            .unwrap_or(plugin.registry.as_str());
        let token = cfg
            .token
            .as_deref()
            .filter(|s| non_blank(s))
            .unwrap_or(plugin.token.as_str());
        if !non_blank(token) {
            return MiddlewareResult::failure(
                "npm authentication token is not specified in both global and local configuration.",
            );
        }

        let npmrc_dir = match prepare_output_dir(ctx.working_dir(), ".moonlit/npm-push") {
            Ok(d) => d,
            Err(e) => {
                return MiddlewareResult::failure(format!("Failed to prepare npm config: {e}"))
            }
        };
        // Best-effort owner-only dir on Unix (the token file below is created 0o600).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&npmrc_dir, std::fs::Permissions::from_mode(0o700));
        }
        if let Err(e) = write_npmrc(&npmrc_dir, &format!("{}\n", npmrc_line(registry, token))) {
            return MiddlewareResult::failure(format!("Failed to write npm config: {e}"));
        }

        let mut args = vec![
            "publish".to_string(),
            cfg.package.clone(),
            "--registry".to_string(),
            registry.to_string(),
            "--tag".to_string(),
            cfg.tag.clone(),
        ];
        if let Some(a) = cfg.access.as_deref().filter(|s| non_blank(s)) {
            args.push("--access".to_string());
            args.push(a.to_string());
        }
        args.push("--userconfig".to_string());
        args.push(".moonlit/npm-push/.npmrc".to_string());

        let result = match npm(ctx, ".").args(args).stream(LineHandler::severity()) {
            Ok(o) if o.success() => MiddlewareResult::success(),
            Ok(o) => {
                let combined = format!("{}\n{}", o.stdout(), o.stderr()).to_ascii_lowercase();
                if combined.contains("epublishconflict")
                    || combined.contains("e409")
                    || combined.contains("409 conflict")
                    || combined.contains("cannot publish over")
                    || combined.contains("previously published versions")
                    || (combined.contains("403")
                        && combined.contains("already")
                        && combined.contains("publish"))
                {
                    MiddlewareResult::failure("Version already published.")
                } else if combined.contains("401 (")
                    || combined.contains("403 (")
                    || combined.contains("e401")
                    || combined.contains("eneedauth")
                    || combined.contains("unauthorized")
                    || combined.contains("forbidden")
                {
                    MiddlewareResult::failure(
                        "Failed to push package: Authentication error. Please check your token and permissions.",
                    )
                } else {
                    MiddlewareResult::failure(format!(
                        "Failed to push package: {}",
                        exit_phrase(o.exit_code)
                    ))
                }
            }
            Err(e) => MiddlewareResult::failure(format!("Failed to push package: {e}")),
        };
        // Best-effort: drop the scoped .npmrc so the auth token does not linger in the
        // working tree after the run (a later broad `git add` could otherwise capture it).
        let _ = std::fs::remove_dir_all(resolve(ctx.working_dir(), ".moonlit/npm-push"));
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_plugin_sdk::process::{OutputChunk, StdioStream};
    use moonlit_plugin_sdk::testing::{run, MockHost};

    fn pkg_dir() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("app-1.0.0.tgz"), b"tgz").unwrap();
        d
    }
    fn ctx_with<'a>(host: &'a MockHost, dir: &std::path::Path, cfg: &'a NodeConfig) -> Context<'a> {
        Context::new(host, dir.to_str().unwrap().into(), "push".into()).with_plugin_config(cfg)
    }
    fn err(text: &str) -> OutputChunk {
        OutputChunk {
            stream: StdioStream::Stderr,
            text: text.to_string(),
        }
    }

    // --- pure npmrc_line ---
    #[test]
    fn npmrc_line_strips_https_and_anchors_slash() {
        assert_eq!(
            npmrc_line("https://registry.npmjs.org", "TOK"),
            "//registry.npmjs.org/:_authToken=TOK"
        );
    }
    #[test]
    fn npmrc_line_strips_http_and_keeps_port() {
        assert_eq!(
            npmrc_line("http://localhost:4873", "TOK"),
            "//localhost:4873/:_authToken=TOK"
        );
    }
    #[test]
    fn npmrc_line_trailing_slash_idempotent() {
        assert_eq!(
            npmrc_line("https://registry.npmjs.org/", "TOK"),
            "//registry.npmjs.org/:_authToken=TOK"
        );
    }

    // --- write_npmrc permissions (Unix) ---
    #[cfg(unix)]
    #[test]
    fn write_npmrc_creates_owner_only_file() {
        use std::os::unix::fs::PermissionsExt;
        let d = tempfile::tempdir().unwrap();
        write_npmrc(d.path(), "//r/:_authToken=SECRET\n").unwrap();
        let mode = std::fs::metadata(d.path().join(".npmrc"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "token file must be owner-only");
    }

    // --- Push middleware ---
    #[test]
    fn publishes_with_config_registry_and_token_off_argv() {
        let d = pkg_dir();
        let host = MockHost::new().with_process_result(0, vec![]);
        let plugin = NodeConfig::default();
        let cfg = PushConfig {
            package: "app-1.0.0.tgz".into(),
            registry: Some("https://feed".into()),
            token: Some("SECRET".into()),
            access: Some("public".into()),
            ..Default::default()
        };
        assert!(run(&Push, &ctx_with(&host, d.path(), &plugin), cfg).is_success());
        let args = &host.recorded_commands()[0].args;
        assert_eq!(
            args,
            &vec![
                "publish",
                "app-1.0.0.tgz",
                "--registry",
                "https://feed",
                "--tag",
                "latest",
                "--access",
                "public",
                "--userconfig",
                ".moonlit/npm-push/.npmrc",
            ]
        );
        assert!(
            !args.iter().any(|a| a == "SECRET"),
            "token must not be on argv"
        );
        // The scoped .npmrc (which held the token) is cleaned up after the run.
        assert!(!d.path().join(".moonlit/npm-push/.npmrc").exists());
    }

    #[test]
    fn falls_back_to_plugin_registry_and_token() {
        let d = pkg_dir();
        let host = MockHost::new().with_process_result(0, vec![]);
        let plugin: NodeConfig = serde_json::from_value(
            serde_json::json!({ "registry": "https://plug", "token": "PTOK" }),
        )
        .unwrap();
        let cfg = PushConfig {
            package: "app-1.0.0.tgz".into(),
            ..Default::default()
        };
        assert!(run(&Push, &ctx_with(&host, d.path(), &plugin), cfg).is_success());
        let args = &host.recorded_commands()[0].args;
        assert_eq!(args[2], "--registry");
        assert_eq!(args[3], "https://plug");
        assert!(!d.path().join(".moonlit/npm-push/.npmrc").exists());
    }

    #[test]
    fn missing_tarball_fails_before_config() {
        let d = tempfile::tempdir().unwrap();
        let host = MockHost::new();
        let plugin = NodeConfig::default();
        let cfg = PushConfig {
            package: "nope.tgz".into(),
            token: Some("T".into()),
            ..Default::default()
        };
        let w = run(&Push, &ctx_with(&host, d.path(), &plugin), cfg).into_wit();
        assert!(w
            .error_message
            .unwrap()
            .starts_with("Package tarball not found at path:"));
    }

    #[test]
    fn blank_token_everywhere_fails() {
        let d = pkg_dir();
        let host = MockHost::new();
        let plugin = NodeConfig::default(); // blank token
        let cfg = PushConfig {
            package: "app-1.0.0.tgz".into(),
            ..Default::default()
        };
        let w = run(&Push, &ctx_with(&host, d.path(), &plugin), cfg).into_wit();
        assert_eq!(
            w.error_message.as_deref(),
            Some(
                "npm authentication token is not specified in both global and local configuration."
            )
        );
        assert!(host.recorded_commands().is_empty());
    }

    #[test]
    fn http_401_maps_to_auth_error() {
        let d = pkg_dir();
        let host = MockHost::new().with_process_result(
            1,
            vec![err("npm error code E401\nnpm error 401 (Unauthorized)")],
        );
        let plugin: NodeConfig =
            serde_json::from_value(serde_json::json!({ "token": "T" })).unwrap();
        let cfg = PushConfig {
            package: "app-1.0.0.tgz".into(),
            ..Default::default()
        };
        let w = run(&Push, &ctx_with(&host, d.path(), &plugin), cfg).into_wit();
        assert_eq!(
            w.error_message.as_deref(),
            Some("Failed to push package: Authentication error. Please check your token and permissions.")
        );
    }

    #[test]
    fn version_conflict_maps_to_already_published() {
        let d = pkg_dir();
        let host = MockHost::new().with_process_result(
            1,
            vec![err("npm error code EPUBLISHCONFLICT\nnpm error cannot publish over previously published versions")],
        );
        let plugin: NodeConfig =
            serde_json::from_value(serde_json::json!({ "token": "T" })).unwrap();
        let cfg = PushConfig {
            package: "app-1.0.0.tgz".into(),
            ..Default::default()
        };
        let w = run(&Push, &ctx_with(&host, d.path(), &plugin), cfg).into_wit();
        assert_eq!(
            w.error_message.as_deref(),
            Some("Version already published.")
        );
    }

    #[test]
    fn http_409_maps_to_already_published() {
        let d = pkg_dir();
        let host = MockHost::new().with_process_result(
            1,
            vec![err(
                "npm error code E409\nnpm error 409 Conflict - PUT https://registry/app",
            )],
        );
        let plugin: NodeConfig =
            serde_json::from_value(serde_json::json!({ "token": "T" })).unwrap();
        let cfg = PushConfig {
            package: "app-1.0.0.tgz".into(),
            ..Default::default()
        };
        let w = run(&Push, &ctx_with(&host, d.path(), &plugin), cfg).into_wit();
        assert_eq!(
            w.error_message.as_deref(),
            Some("Version already published.")
        );
    }

    #[test]
    fn bare_401_in_filename_is_not_auth() {
        // A `401` only inside a token (a version) must fall through to generic, not auth.
        let d = pkg_dir();
        let host = MockHost::new().with_process_result(
            1,
            vec![err("npm error publishing app-1.401.0.tgz: network reset")],
        );
        let plugin: NodeConfig =
            serde_json::from_value(serde_json::json!({ "token": "T" })).unwrap();
        let cfg = PushConfig {
            package: "app-1.0.0.tgz".into(),
            ..Default::default()
        };
        let w = run(&Push, &ctx_with(&host, d.path(), &plugin), cfg).into_wit();
        assert_eq!(
            w.error_message.as_deref(),
            Some("Failed to push package: Npm command failed with exit code 1")
        );
    }
}
