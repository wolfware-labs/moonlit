//! `dotnet test` — run tests, parse TRX counters into pass/fail/skip/total outputs.

use crate::dotnet::{dotnet, exit_phrase, prepare_output_dir, project_slug, resolve};
use crate::trx::{parse_counters, TrxCounters};
use moonlit_sdk::prelude::*;
use moonlit_sdk::process::LineHandler;

/// Decide the middleware result from the process exit code and the parsed TRX counters
/// (`None` = results file absent/unparseable). Pure — unit-testable without a subprocess.
fn outcome(exit_code: i32, counters: Option<TrxCounters>) -> MiddlewareResult {
    match counters {
        None => {
            if exit_code == 0 {
                MiddlewareResult::failure("Test results file was not produced.")
            } else {
                MiddlewareResult::failure(format!(
                    "Failed to run tests: {}",
                    exit_phrase(exit_code)
                ))
            }
        }
        Some(c) => {
            if exit_code != 0 && c.failed > 0 {
                MiddlewareResult::failure(format!("{} test(s) failed.", c.failed))
            } else if exit_code != 0 {
                MiddlewareResult::failure(format!(
                    "Failed to run tests: {}",
                    exit_phrase(exit_code)
                ))
            } else {
                MiddlewareResult::success_with(|o| {
                    o.set("passed", c.passed);
                    o.set("failed", c.failed);
                    o.set("skipped", c.skipped);
                    o.set("total", c.total);
                })
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TestConfig {
    pub project: String,
    pub configuration: String,
    pub filter: Option<String>,
    pub no_build: bool,
    pub collect_coverage: bool,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            project: String::new(),
            configuration: "Release".to_string(),
            filter: None,
            no_build: false,
            collect_coverage: false,
        }
    }
}

#[derive(Default)]
pub struct Test;

impl Middleware for Test {
    const NAME: &'static str = "test";
    const DESCRIPTION: &'static str = "run .NET tests and report pass/fail/skip counts";
    type Config = TestConfig;

    fn execute(&self, ctx: &Context, cfg: TestConfig) -> MiddlewareResult {
        let proj_path = resolve(ctx.working_dir(), &cfg.project);
        if !proj_path.is_file() {
            return MiddlewareResult::failure(format!(
                "Project file not found at path: {}",
                proj_path.display()
            ));
        }
        let results_rel = format!(".moonlit/dotnet-test/{}", project_slug(&cfg.project));
        let results_dir = match prepare_output_dir(ctx.working_dir(), &results_rel) {
            Ok(d) => d,
            Err(e) => {
                return MiddlewareResult::failure(format!(
                    "Failed to prepare results directory: {e}"
                ))
            }
        };

        let mut args: Vec<String> = vec![
            "test".to_string(),
            cfg.project.clone(),
            "--configuration".to_string(),
            cfg.configuration.clone(),
        ];
        if let Some(f) = cfg.filter.as_ref().filter(|s| !s.trim().is_empty()) {
            args.push("--filter".to_string());
            args.push(f.clone());
        }
        if cfg.no_build {
            args.push("--no-build".to_string());
        }
        args.push("--logger".to_string());
        args.push("trx;LogFileName=moonlit.trx".to_string());
        args.push("--results-directory".to_string());
        args.push(results_rel.clone());
        if cfg.collect_coverage {
            args.push("--collect".to_string());
            args.push("XPlat Code Coverage".to_string());
        }

        let out = match dotnet(ctx).args(args).stream(LineHandler::severity()) {
            Ok(o) => o,
            Err(e) => return MiddlewareResult::failure(format!("Failed to run tests: {e}")),
        };

        let trx_path = results_dir.join("moonlit.trx");
        let counters = std::fs::read_to_string(&trx_path)
            .ok()
            .and_then(|s| parse_counters(&s));

        outcome(out.exit_code, counters)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_sdk::testing::{run, MockHost};

    fn proj_dir() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("Tests.csproj"), b"<Project/>").unwrap();
        d
    }
    fn counters(passed: u32, failed: u32, skipped: u32, total: u32) -> TrxCounters {
        TrxCounters {
            passed,
            failed,
            skipped,
            total,
        }
    }

    // --- outcome (pure) ---
    #[test]
    fn outcome_success_emits_counts() {
        let w = outcome(0, Some(counters(8, 1, 1, 10))).into_wit();
        assert!(w.successful);
        let get = |k: &str| {
            w.output
                .iter()
                .find(|(n, _)| n == k)
                .map(|(_, v)| v.clone())
                .unwrap()
        };
        assert_eq!(get("passed"), "8");
        assert_eq!(get("failed"), "1");
        assert_eq!(get("skipped"), "1");
        assert_eq!(get("total"), "10");
    }
    #[test]
    fn outcome_failed_tests_report_count() {
        let w = outcome(1, Some(counters(3, 2, 0, 5))).into_wit();
        assert!(!w.successful);
        assert_eq!(w.error_message.as_deref(), Some("2 test(s) failed."));
    }
    #[test]
    fn outcome_non_zero_without_failures_is_generic() {
        let w = outcome(1, Some(counters(4, 0, 0, 4))).into_wit();
        assert_eq!(
            w.error_message.as_deref(),
            Some("Failed to run tests: Dotnet command failed with exit code 1")
        );
    }
    #[test]
    fn outcome_missing_trx_on_success_fails() {
        let w = outcome(0, None).into_wit();
        assert_eq!(
            w.error_message.as_deref(),
            Some("Test results file was not produced.")
        );
    }
    #[test]
    fn outcome_missing_trx_on_failure_is_generic() {
        let w = outcome(1, None).into_wit();
        assert_eq!(
            w.error_message.as_deref(),
            Some("Failed to run tests: Dotnet command failed with exit code 1")
        );
    }

    // --- Test middleware ---
    // MockHost's `dotnet test` writes no TRX and prepare_output_dir wipes the results dir,
    // so a run resolves to the missing-TRX path; the argv tests assert the recorded command
    // (captured during `.stream()`) and ignore the run's result.
    #[test]
    fn builds_full_argv_with_all_flags() {
        let d = proj_dir();
        let host = MockHost::new().with_process_result(0, vec![]);
        let ctx = Context::new(&host, d.path().to_str().unwrap().into(), "test".into());
        let cfg = TestConfig {
            project: "Tests.csproj".into(),
            filter: Some("Category=Unit".into()),
            no_build: true,
            collect_coverage: true,
            ..Default::default()
        };
        let _ = run(&Test, &ctx, cfg);
        let cmds = host.recorded_commands();
        assert_eq!(
            cmds[0].args,
            vec![
                "test",
                "Tests.csproj",
                "--configuration",
                "Release",
                "--filter",
                "Category=Unit",
                "--no-build",
                "--logger",
                "trx;LogFileName=moonlit.trx",
                "--results-directory",
                ".moonlit/dotnet-test/Tests",
                "--collect",
                "XPlat Code Coverage",
            ]
        );
    }
    #[test]
    fn minimal_argv_omits_optional_flags() {
        let d = proj_dir();
        let host = MockHost::new().with_process_result(0, vec![]);
        let ctx = Context::new(&host, d.path().to_str().unwrap().into(), "test".into());
        let cfg = TestConfig {
            project: "Tests.csproj".into(),
            ..Default::default()
        };
        let _ = run(&Test, &ctx, cfg);
        let cmds = host.recorded_commands();
        assert_eq!(
            cmds[0].args,
            vec![
                "test",
                "Tests.csproj",
                "--configuration",
                "Release",
                "--logger",
                "trx;LogFileName=moonlit.trx",
                "--results-directory",
                ".moonlit/dotnet-test/Tests",
            ]
        );
    }
    #[test]
    fn missing_project_fails() {
        let d = tempfile::tempdir().unwrap();
        let host = MockHost::new();
        let ctx = Context::new(&host, d.path().to_str().unwrap().into(), "test".into());
        let cfg = TestConfig {
            project: "nope.csproj".into(),
            ..Default::default()
        };
        let w = run(&Test, &ctx, cfg).into_wit();
        assert!(w
            .error_message
            .unwrap()
            .starts_with("Project file not found at path:"));
    }
}
