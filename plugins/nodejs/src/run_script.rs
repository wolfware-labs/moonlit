//! `npm run <script>` — run a package.json script, with missing-script detection.

use crate::npm::{exit_phrase, npm, require_package_json};
use moonlit_sdk::prelude::*;
use moonlit_sdk::process::LineHandler;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RunScriptConfig {
    pub directory: String,
    pub script: String,
    pub args: Vec<String>,
}

impl Default for RunScriptConfig {
    fn default() -> Self {
        Self {
            directory: ".".to_string(),
            script: String::new(),
            args: Vec::new(),
        }
    }
}

#[derive(Default)]
pub struct RunScript;

impl Middleware for RunScript {
    const NAME: &'static str = "run-script";
    const DESCRIPTION: &'static str = "run a package.json script via npm run";
    type Config = RunScriptConfig;

    fn execute(&self, ctx: &Context, cfg: RunScriptConfig) -> MiddlewareResult {
        if let Err(msg) = require_package_json(ctx.working_dir(), &cfg.directory) {
            return MiddlewareResult::failure(msg);
        }
        let mut args = vec!["run".to_string(), cfg.script.clone()];
        if !cfg.args.is_empty() {
            args.push("--".to_string());
            args.extend(cfg.args.iter().cloned());
        }
        match npm(ctx, &cfg.directory)
            .args(args)
            .stream(LineHandler::severity())
        {
            Ok(o) if o.success() => MiddlewareResult::success(),
            Ok(o) => {
                let combined = format!("{}\n{}", o.stdout(), o.stderr()).to_ascii_lowercase();
                if combined.contains("missing script:") {
                    MiddlewareResult::failure(format!(
                        "Script '{}' not found in package.json.",
                        cfg.script
                    ))
                } else {
                    MiddlewareResult::failure(format!(
                        "Failed to run script: {}",
                        exit_phrase(o.exit_code)
                    ))
                }
            }
            Err(e) => MiddlewareResult::failure(format!("Failed to run script: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_sdk::process::{OutputChunk, StdioStream};
    use moonlit_sdk::testing::{run, MockHost};

    fn proj_dir() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("package.json"), b"{}").unwrap();
        d
    }
    fn ctx<'a>(host: &'a MockHost, dir: &std::path::Path) -> Context<'a> {
        Context::new(host, dir.to_str().unwrap().into(), "run-script".into())
    }
    fn err(text: &str) -> OutputChunk {
        OutputChunk {
            stream: StdioStream::Stderr,
            text: text.to_string(),
        }
    }

    #[test]
    fn builds_run_argv_with_forwarded_args() {
        let d = proj_dir();
        let host = MockHost::new().with_process_result(0, vec![]);
        let cfg = RunScriptConfig {
            script: "lint".into(),
            args: vec!["--fix".into(), "src".into()],
            ..Default::default()
        };
        let _ = run(&RunScript, &ctx(&host, d.path()), cfg);
        assert_eq!(
            host.recorded_commands()[0].args,
            vec!["run", "lint", "--", "--fix", "src"]
        );
    }

    #[test]
    fn no_args_omits_double_dash() {
        let d = proj_dir();
        let host = MockHost::new().with_process_result(0, vec![]);
        let cfg = RunScriptConfig {
            script: "build".into(),
            ..Default::default()
        };
        let _ = run(&RunScript, &ctx(&host, d.path()), cfg);
        assert_eq!(host.recorded_commands()[0].args, vec!["run", "build"]);
    }

    #[test]
    fn missing_script_maps_to_frozen_message() {
        let d = proj_dir();
        let host = MockHost::new()
            .with_process_result(1, vec![err("npm error Missing script: \"deploy\"")]);
        let cfg = RunScriptConfig {
            script: "deploy".into(),
            ..Default::default()
        };
        let w = run(&RunScript, &ctx(&host, d.path()), cfg).into_wit();
        assert_eq!(
            w.error_message.as_deref(),
            Some("Script 'deploy' not found in package.json.")
        );
    }

    #[test]
    fn other_non_zero_maps_to_generic() {
        let d = proj_dir();
        let host = MockHost::new().with_process_result(2, vec![err("build failed: TS2304")]);
        let cfg = RunScriptConfig {
            script: "build".into(),
            ..Default::default()
        };
        let w = run(&RunScript, &ctx(&host, d.path()), cfg).into_wit();
        assert_eq!(
            w.error_message.as_deref(),
            Some("Failed to run script: Npm command failed with exit code 2")
        );
    }

    #[test]
    fn missing_package_json_fails_before_spawn() {
        let d = tempfile::tempdir().unwrap();
        let host = MockHost::new();
        let cfg = RunScriptConfig {
            script: "build".into(),
            ..Default::default()
        };
        let w = run(&RunScript, &ctx(&host, d.path()), cfg).into_wit();
        assert!(w
            .error_message
            .unwrap()
            .starts_with("package.json not found in directory:"));
        assert!(host.recorded_commands().is_empty());
    }
}
