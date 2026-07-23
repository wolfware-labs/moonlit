//! `docker compose up` over `DOCKER_HOST` — the compose deploy path. Swarm
//! (`service`) is an explicit unsupported stub for MVP.

use crate::docker::{docker, fail};
use moonlit_plugin_sdk::prelude::*;
use moonlit_plugin_sdk::process::LineHandler;
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DeployConfig {
    pub image: String,
    pub host: String,
    pub compose_file: Option<String>,
    pub service: Option<String>,
    pub environment: BTreeMap<String, String>,
    pub pull: bool,
}

impl Default for DeployConfig {
    fn default() -> Self {
        Self {
            image: String::new(),
            host: String::new(),
            compose_file: None,
            service: None,
            environment: BTreeMap::new(),
            pull: true,
        }
    }
}

#[derive(Default)]
pub struct Deploy;

impl Middleware for Deploy {
    const NAME: &'static str = "deploy";
    const DESCRIPTION: &'static str = "deploy an image via docker compose";
    type Config = DeployConfig;

    fn execute(&self, ctx: &Context, cfg: DeployConfig) -> MiddlewareResult {
        // Swarm path (service) is the unsupported stub — check first.
        if cfg
            .service
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .is_some()
        {
            return MiddlewareResult::failure("Swarm deploys are not supported yet.");
        }
        let compose_file = match cfg.compose_file.as_deref().filter(|f| !f.trim().is_empty()) {
            Some(f) => f,
            None => {
                return MiddlewareResult::failure(
                    "A compose file is required for a compose deployment.",
                )
            }
        };
        let mut c = docker(ctx)
            .env("DOCKER_HOST", &cfg.host)
            .arg("compose")
            .arg("-f")
            .arg(compose_file)
            .arg("up")
            .arg("-d");
        if cfg.pull {
            c = c.arg("--pull").arg("always");
        }
        for (k, v) in &cfg.environment {
            c = c.env(k, v);
        }
        match c.stream(LineHandler::severity()) {
            Ok(o) if o.success() => MiddlewareResult::success(),
            Ok(o) => fail("deploy with docker compose", o.exit_code),
            Err(e) => {
                MiddlewareResult::failure(format!("Failed to deploy with docker compose: {e}"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_plugin_sdk::testing::{run, MockHost};

    fn ctx<'a>(host: &'a MockHost) -> Context<'a> {
        Context::new(host, "/wd".into(), "deploy".into())
    }

    #[test]
    fn service_set_is_swarm_unsupported_before_spawn() {
        let host = MockHost::new();
        let cfg = DeployConfig {
            host: "ssh://h".into(),
            service: Some("web".into()),
            ..Default::default()
        };
        let w = run(&Deploy, &ctx(&host), cfg).into_wit();
        assert_eq!(
            w.error_message.as_deref(),
            Some("Swarm deploys are not supported yet.")
        );
        assert!(host.recorded_commands().is_empty());
    }

    #[test]
    fn missing_compose_file_fails_before_spawn() {
        let host = MockHost::new();
        let cfg = DeployConfig {
            host: "ssh://h".into(),
            ..Default::default()
        };
        let w = run(&Deploy, &ctx(&host), cfg).into_wit();
        assert_eq!(
            w.error_message.as_deref(),
            Some("A compose file is required for a compose deployment.")
        );
        assert!(host.recorded_commands().is_empty());
    }

    #[test]
    fn compose_path_sets_docker_host_env_and_pull_always() {
        let host = MockHost::new().with_process_result(0, vec![]);
        let mut environment = BTreeMap::new();
        environment.insert("TAG".into(), "v1".into());
        let cfg = DeployConfig {
            host: "ssh://user@h".into(),
            compose_file: Some("docker-compose.yml".into()),
            environment,
            pull: true,
            ..Default::default()
        };
        assert!(run(&Deploy, &ctx(&host), cfg).is_success());
        let cmd = &host.recorded_commands()[0];
        assert_eq!(
            cmd.args,
            vec![
                "compose",
                "-f",
                "docker-compose.yml",
                "up",
                "-d",
                "--pull",
                "always"
            ]
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>()
        );
        assert!(cmd
            .env
            .contains(&("DOCKER_HOST".to_string(), "ssh://user@h".to_string())));
        assert!(cmd.env.contains(&("TAG".to_string(), "v1".to_string())));
    }

    #[test]
    fn pull_false_omits_pull_always() {
        let host = MockHost::new().with_process_result(0, vec![]);
        let cfg = DeployConfig {
            host: "ssh://h".into(),
            compose_file: Some("c.yml".into()),
            pull: false,
            ..Default::default()
        };
        let _ = run(&Deploy, &ctx(&host), cfg);
        assert!(!host.recorded_commands()[0]
            .args
            .iter()
            .any(|a| a == "--pull"));
    }

    #[test]
    fn non_zero_exit_maps_to_deploy_failure() {
        let host = MockHost::new().with_process_result(1, vec![]);
        let cfg = DeployConfig {
            host: "ssh://h".into(),
            compose_file: Some("c.yml".into()),
            ..Default::default()
        };
        let w = run(&Deploy, &ctx(&host), cfg).into_wit();
        assert_eq!(
            w.error_message.as_deref(),
            Some("Failed to deploy with docker compose: Docker command failed with exit code 1")
        );
    }
}
