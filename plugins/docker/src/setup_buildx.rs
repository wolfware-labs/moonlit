//! `docker buildx create` — create a buildx builder; optionally record its name
//! in plugin shared state (and emit it as the `name` output) for `build-and-push`.

use crate::docker::{docker, fail};
use crate::state::DockerShared;
use moonlit_sdk::prelude::*;
use moonlit_sdk::process::LineHandler;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SetupBuildxConfig {
    pub name: Option<String>,
    pub driver: String,
    pub endpoint: Option<String>,
    pub bootstrap: bool,
    pub set_builder_variable: bool,
    pub platforms: Vec<String>,
}

impl Default for SetupBuildxConfig {
    fn default() -> Self {
        Self {
            name: None,
            driver: "docker-container".to_string(),
            endpoint: None,
            bootstrap: true,
            set_builder_variable: true,
            platforms: Vec::new(),
        }
    }
}

#[derive(Default)]
pub struct SetupBuildx;

impl Middleware for SetupBuildx {
    const NAME: &'static str = "setup-buildx";
    const DESCRIPTION: &'static str = "create a docker buildx builder";
    type Config = SetupBuildxConfig;

    fn execute(&self, ctx: &Context, cfg: SetupBuildxConfig) -> MiddlewareResult {
        let name = match cfg.name.as_deref().filter(|n| !n.trim().is_empty()) {
            Some(n) => n.to_string(),
            None => format!("moonlit-builder-{}", ctx.random().uuid()),
        };
        let mut c = docker(ctx)
            .arg("buildx")
            .arg("create")
            .arg("--name")
            .arg(&name)
            .arg("--driver")
            .arg(&cfg.driver);
        if cfg.bootstrap {
            c = c.arg("--bootstrap");
        }
        for p in &cfg.platforms {
            c = c.arg("--platform").arg(p);
        }
        if let Some(ep) = cfg.endpoint.as_deref().filter(|e| !e.trim().is_empty()) {
            c = c.arg(ep);
        }
        match c.stream(LineHandler::severity()) {
            Ok(o) if o.success() => {
                if cfg.set_builder_variable {
                    ctx.state::<DockerShared>().builder.set(Some(name.clone()));
                }
                MiddlewareResult::success_with(|o| o.set("name", name))
            }
            Ok(o) => fail("create buildx builder", o.exit_code),
            Err(e) => MiddlewareResult::failure(format!("Failed to create buildx builder: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_sdk::testing::{run, MockHost};

    fn ctx<'a>(host: &'a MockHost, shared: &'a DockerShared) -> Context<'a> {
        Context::new(host, "/wd".into(), "setup-buildx".into()).with_state(shared)
    }

    #[test]
    fn explicit_name_builds_full_argv_in_order() {
        let host = MockHost::new().with_process_result(0, vec![]);
        let shared = DockerShared::default();
        let cfg = SetupBuildxConfig {
            name: Some("mybuilder".into()),
            driver: "docker-container".into(),
            endpoint: Some("ssh://host".into()),
            bootstrap: true,
            set_builder_variable: true,
            platforms: vec!["linux/amd64".into(), "linux/arm64".into()],
        };
        assert!(run(&SetupBuildx, &ctx(&host, &shared), cfg).is_success());
        assert_eq!(
            host.recorded_commands()[0].args,
            vec![
                "buildx",
                "create",
                "--name",
                "mybuilder",
                "--driver",
                "docker-container",
                "--bootstrap",
                "--platform",
                "linux/amd64",
                "--platform",
                "linux/arm64",
                "ssh://host",
            ]
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>()
        );
    }

    #[test]
    fn bootstrap_false_omits_flag() {
        let host = MockHost::new().with_process_result(0, vec![]);
        let shared = DockerShared::default();
        let cfg = SetupBuildxConfig {
            name: Some("b".into()),
            bootstrap: false,
            ..Default::default()
        };
        let _ = run(&SetupBuildx, &ctx(&host, &shared), cfg);
        assert!(!host.recorded_commands()[0]
            .args
            .iter()
            .any(|a| a == "--bootstrap"));
    }

    #[test]
    fn default_name_uses_uuid_and_sets_state_and_output() {
        let host = MockHost::new()
            .with_random(&[0xab])
            .with_process_result(0, vec![]);
        let shared = DockerShared::default();
        let expected = "moonlit-builder-abababab-abab-4bab-abab-abababababab";
        let w = run(
            &SetupBuildx,
            &ctx(&host, &shared),
            SetupBuildxConfig::default(),
        )
        .into_wit();
        assert!(w.successful);
        // --name argument is the generated builder name
        let args = &host.recorded_commands()[0].args;
        let name_idx = args.iter().position(|a| a == "--name").unwrap();
        assert_eq!(args[name_idx + 1], expected);
        // state recorded
        assert_eq!(shared.builder.get(), Some(expected.to_string()));
        // output `name` is the same value, JSON-quoted
        let out: std::collections::HashMap<_, _> = w.output.into_iter().collect();
        assert_eq!(out["name"], format!("\"{expected}\""));
    }

    #[test]
    fn set_builder_variable_false_leaves_state_none() {
        let host = MockHost::new().with_process_result(0, vec![]);
        let shared = DockerShared::default();
        let cfg = SetupBuildxConfig {
            name: Some("b".into()),
            set_builder_variable: false,
            ..Default::default()
        };
        let _ = run(&SetupBuildx, &ctx(&host, &shared), cfg);
        assert_eq!(shared.builder.get(), None);
    }

    #[test]
    fn non_zero_exit_fails_without_state_or_output() {
        let host = MockHost::new().with_process_result(1, vec![]);
        let shared = DockerShared::default();
        let cfg = SetupBuildxConfig {
            name: Some("b".into()),
            ..Default::default()
        };
        let w = run(&SetupBuildx, &ctx(&host, &shared), cfg).into_wit();
        assert_eq!(
            w.error_message.as_deref(),
            Some("Failed to create buildx builder: Docker command failed with exit code 1")
        );
        assert_eq!(shared.builder.get(), None);
        assert!(w.output.is_empty());
    }
}
