//! `npm run <script>` for tests — run the test script, fail on non-zero.

use crate::npm::{npm, require_package_json};
use moonlit_plugin_sdk::prelude::*;
use moonlit_plugin_sdk::process::LineHandler;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TestConfig {
    pub directory: String,
    pub script: String,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            directory: ".".to_string(),
            script: "test".to_string(),
        }
    }
}

#[derive(Default)]
pub struct Test;

impl Middleware for Test {
    const NAME: &'static str = "test";
    const DESCRIPTION: &'static str = "run the npm test script";
    type Config = TestConfig;

    fn execute(&self, ctx: &Context, cfg: TestConfig) -> MiddlewareResult {
        if let Err(msg) = require_package_json(ctx.working_dir(), &cfg.directory) {
            return MiddlewareResult::failure(msg);
        }
        let args = vec!["run".to_string(), cfg.script.clone()];
        match npm(ctx, &cfg.directory)
            .args(args)
            .stream(LineHandler::severity())
        {
            Ok(o) if o.success() => MiddlewareResult::success(),
            Ok(_) => MiddlewareResult::failure("Tests failed."),
            Err(e) => MiddlewareResult::failure(format!("Failed to run tests: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_plugin_sdk::testing::{run, MockHost};

    fn proj_dir() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("package.json"), b"{}").unwrap();
        d
    }
    fn ctx<'a>(host: &'a MockHost, dir: &std::path::Path) -> Context<'a> {
        Context::new(host, dir.to_str().unwrap().into(), "test".into())
    }

    #[test]
    fn default_runs_npm_run_test() {
        let d = proj_dir();
        let host = MockHost::new().with_process_result(0, vec![]);
        assert!(run(&Test, &ctx(&host, d.path()), TestConfig::default()).is_success());
        assert_eq!(host.recorded_commands()[0].args, vec!["run", "test"]);
    }

    #[test]
    fn custom_script_runs() {
        let d = proj_dir();
        let host = MockHost::new().with_process_result(0, vec![]);
        let cfg = TestConfig {
            script: "test:ci".into(),
            ..Default::default()
        };
        let _ = run(&Test, &ctx(&host, d.path()), cfg);
        assert_eq!(host.recorded_commands()[0].args, vec!["run", "test:ci"]);
    }

    #[test]
    fn non_zero_maps_to_tests_failed() {
        let d = proj_dir();
        let host = MockHost::new().with_process_result(1, vec![]);
        let w = run(&Test, &ctx(&host, d.path()), TestConfig::default()).into_wit();
        assert_eq!(w.error_message.as_deref(), Some("Tests failed."));
    }

    #[test]
    fn missing_package_json_fails_before_spawn() {
        let d = tempfile::tempdir().unwrap();
        let host = MockHost::new();
        let w = run(&Test, &ctx(&host, d.path()), TestConfig::default()).into_wit();
        assert!(w
            .error_message
            .unwrap()
            .starts_with("package.json not found in directory:"));
        assert!(host.recorded_commands().is_empty());
    }
}
