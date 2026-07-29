use moonlit_sdk::prelude::*;
use std::collections::BTreeMap;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct RunModulesConfig {
    pub module_paths: Vec<String>,
    pub stages: Vec<String>,
    pub continue_on_module_error: bool,
    pub arguments: BTreeMap<String, String>,
}

/// Split a module path into `(-w dir, Option<-f basename>)`. A `.yml`/`.yaml`
/// path (case-insensitive) is a file: dir is its parent (or `.`), file is the
/// basename. Anything else is a directory: dir is the path, no `-f`.
fn split_path(path: &str) -> (&str, Option<&str>) {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".yml") || lower.ends_with(".yaml") {
        match path.rfind('/') {
            Some(i) => (&path[..i], Some(&path[i + 1..])),
            None => (".", Some(path)),
        }
    } else {
        (path, None)
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ModuleResult {
    module: String,
    successful: bool,
    duration_ms: u64,
}

#[derive(Default)]
pub struct RunModules;

impl Middleware for RunModules {
    const NAME: &'static str = "run-modules";
    const DESCRIPTION: &'static str = "run nested Moonlit release files as modules";
    type Config = RunModulesConfig;

    fn execute(&self, ctx: &Context, cfg: RunModulesConfig) -> MiddlewareResult {
        if cfg.module_paths.is_empty() {
            return MiddlewareResult::failure("No module paths provided for run-modules.");
        }
        let mut results = Vec::with_capacity(cfg.module_paths.len());
        let mut failed_count: u32 = 0;
        for path in &cfg.module_paths {
            let (dir, file) = split_path(path);
            let timer = ctx.clock().start();
            let mut cmd = ctx
                .command("moonlit")
                .cwd(ctx.working_dir())
                .arg("run")
                .arg("-w")
                .arg(dir);
            if let Some(f) = file {
                cmd = cmd.arg("-f").arg(f);
            }
            cmd = cmd.arg("--output").arg("plain");
            for s in &cfg.stages {
                cmd = cmd.arg("-s").arg(s);
            }
            for (k, v) in &cfg.arguments {
                cmd = cmd.arg("-a").arg(format!("{k}={v}"));
            }
            let out = match cmd.stream(LineHandler::severity()) {
                Ok(o) => o,
                Err(e) => return MiddlewareResult::failure(e),
            };
            let duration_ms = timer.elapsed_ms();
            let successful = out.success();
            results.push(ModuleResult {
                module: path.clone(),
                successful,
                duration_ms,
            });
            if !successful {
                failed_count += 1;
                if !cfg.continue_on_module_error {
                    return MiddlewareResult::failure(format!(
                        "Module '{path}' failed with exit code {}.",
                        out.exit_code
                    ));
                }
            }
        }
        MiddlewareResult::success_with(|o| {
            o.set("results", &results);
            o.set("failedCount", failed_count);
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_sdk::process::{OutputChunk, StdioStream};
    use moonlit_sdk::testing::{run, MockHost};

    fn ok_line(text: &str) -> OutputChunk {
        OutputChunk {
            stream: StdioStream::Stdout,
            text: text.to_string(),
        }
    }

    #[test]
    fn directory_path_maps_to_working_dir_only() {
        assert_eq!(split_path("modules/foo"), ("modules/foo", None));
    }

    #[test]
    fn yml_path_splits_into_dir_and_basename() {
        assert_eq!(
            split_path("modules/foo/release.yml"),
            ("modules/foo", Some("release.yml"))
        );
        assert_eq!(split_path("a/b/c.yaml"), ("a/b", Some("c.yaml")));
    }

    #[test]
    fn yml_path_without_slash_uses_dot_dir() {
        assert_eq!(split_path("release.yml"), (".", Some("release.yml")));
    }

    #[test]
    fn yml_extension_match_is_case_insensitive() {
        assert_eq!(split_path("x/Deploy.YML"), ("x", Some("Deploy.YML")));
    }

    #[test]
    fn empty_module_paths_fails_before_any_spawn() {
        let host = MockHost::new();
        let ctx = Context::new(&host, "/w".into(), "modules".into());
        let cfg = RunModulesConfig::default(); // module_paths empty
        let r = run(&RunModules, &ctx, cfg);
        assert!(!r.is_success());
        assert_eq!(
            r.error_message(),
            Some("No module paths provided for run-modules.")
        );
        assert!(host.recorded_commands().is_empty());
    }

    #[test]
    fn builds_run_command_per_module_with_flags() {
        let host = MockHost::new()
            .with_process_result(0, vec![ok_line("done")])
            .with_clock(&[0, 1_000_000]);
        let ctx = Context::new(&host, "/repo".into(), "modules".into());
        let mut cfg = RunModulesConfig {
            module_paths: vec!["modules/foo/release.yml".to_string()],
            stages: vec!["release".to_string()],
            ..Default::default()
        };
        cfg.arguments.insert("key".to_string(), "val".to_string());
        let r = run(&RunModules, &ctx, cfg);
        assert!(r.is_success());
        let cmds = host.recorded_commands();
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].program, "moonlit");
        assert_eq!(cmds[0].cwd.as_deref(), Some("/repo"));
        assert_eq!(
            cmds[0].args,
            vec![
                "run",
                "-w",
                "modules/foo",
                "-f",
                "release.yml",
                "--output",
                "plain",
                "-s",
                "release",
                "-a",
                "key=val",
            ]
        );
    }

    #[test]
    fn directory_module_omits_file_flag() {
        let host = MockHost::new()
            .with_process_result(0, vec![ok_line("done")])
            .with_clock(&[0, 0]);
        let ctx = Context::new(&host, "/repo".into(), "modules".into());
        let cfg = RunModulesConfig {
            module_paths: vec!["modules/foo".to_string()],
            ..Default::default()
        };
        let r = run(&RunModules, &ctx, cfg);
        assert!(r.is_success());
        assert_eq!(
            host.recorded_commands()[0].args,
            vec!["run", "-w", "modules/foo", "--output", "plain"]
        );
    }

    #[test]
    fn fail_fast_stops_at_first_failure() {
        let host = MockHost::new()
            .with_process_result(0, vec![ok_line("ok")])
            .with_process_result(2, vec![ok_line("boom")])
            .with_process_result(0, vec![ok_line("never")])
            .with_clock(&[0, 1, 0, 1]);
        let ctx = Context::new(&host, "/repo".into(), "modules".into());
        let cfg = RunModulesConfig {
            module_paths: vec!["a".to_string(), "b".to_string(), "c".to_string()],
            ..Default::default()
        };
        let r = run(&RunModules, &ctx, cfg);
        assert!(!r.is_success());
        assert_eq!(
            r.error_message(),
            Some("Module 'b' failed with exit code 2.")
        );
        assert_eq!(
            host.recorded_commands().len(),
            2,
            "third module not spawned"
        );
    }

    #[test]
    fn continue_on_error_runs_all_and_reports_failed_count() {
        let host = MockHost::new()
            .with_process_result(0, vec![ok_line("ok")])
            .with_process_result(2, vec![ok_line("boom")])
            .with_process_result(0, vec![ok_line("ok")])
            .with_clock(&[0, 5_000_000, 0, 1_000_000, 0, 2_000_000]);
        let ctx = Context::new(&host, "/repo".into(), "modules".into());
        let cfg = RunModulesConfig {
            module_paths: vec!["a".to_string(), "b".to_string(), "c".to_string()],
            continue_on_module_error: true,
            ..Default::default()
        };
        let w = run(&RunModules, &ctx, cfg).into_wit();
        assert!(w.successful);
        assert_eq!(host.recorded_commands().len(), 3);
        let m: std::collections::HashMap<_, _> = w.output.into_iter().collect();
        assert_eq!(m["failedCount"], "1");
        let results: serde_json::Value = serde_json::from_str(&m["results"]).unwrap();
        assert_eq!(results[0]["module"], "a");
        assert_eq!(results[0]["successful"], true);
        assert_eq!(results[0]["durationMs"], 5);
        assert_eq!(results[1]["successful"], false);
    }
}
