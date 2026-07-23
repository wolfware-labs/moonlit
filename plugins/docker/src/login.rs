//! `docker login` — authenticate to a registry, password fed via stdin.

use crate::docker::{docker, fail};
use moonlit_plugin_sdk::prelude::*;
use moonlit_plugin_sdk::process::LineHandler;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct LoginConfig {
    pub registry: Option<String>,
    pub username: String,
    pub password: String,
}

#[derive(Default)]
pub struct Login;

impl Middleware for Login {
    const NAME: &'static str = "login";
    const DESCRIPTION: &'static str = "authenticate to a Docker registry";
    type Config = LoginConfig;

    fn execute(&self, ctx: &Context, cfg: LoginConfig) -> MiddlewareResult {
        if cfg.username.trim().is_empty() || cfg.password.trim().is_empty() {
            return MiddlewareResult::failure(
                "Docker login requires both username and password to be set.",
            );
        }
        let mut c = docker(ctx).arg("login");
        if let Some(reg) = cfg.registry.as_deref().filter(|r| !r.trim().is_empty()) {
            c = c.arg(reg);
        }
        c = c
            .arg("--username")
            .arg(&cfg.username)
            .arg("--password-stdin")
            .stdin(cfg.password);
        match c.stream(LineHandler::severity()) {
            Ok(o) if o.success() => MiddlewareResult::success(),
            Ok(o) => fail("log in to Docker registry", o.exit_code),
            Err(e) => {
                MiddlewareResult::failure(format!("Failed to log in to Docker registry: {e}"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_plugin_sdk::testing::{run, MockHost};

    fn ctx<'a>(host: &'a MockHost) -> Context<'a> {
        Context::new(host, "/wd".into(), "login".into())
    }

    #[test]
    fn blank_username_or_password_fails_before_spawn() {
        let host = MockHost::new();
        let cfg = LoginConfig {
            username: "  ".into(),
            password: "pw".into(),
            ..Default::default()
        };
        let w = run(&Login, &ctx(&host), cfg).into_wit();
        assert_eq!(
            w.error_message.as_deref(),
            Some("Docker login requires both username and password to be set.")
        );
        assert!(host.recorded_commands().is_empty());
    }

    #[test]
    fn with_registry_builds_positional_and_feeds_password_stdin() {
        let host = MockHost::new().with_process_result(0, vec![]);
        let cfg = LoginConfig {
            registry: Some("registry.example.com".into()),
            username: "u".into(),
            password: "secret".into(),
        };
        assert!(run(&Login, &ctx(&host), cfg).is_success());
        let cmd = &host.recorded_commands()[0];
        assert_eq!(
            cmd.args,
            vec![
                "login".to_string(),
                "registry.example.com".to_string(),
                "--username".to_string(),
                "u".to_string(),
                "--password-stdin".to_string(),
            ]
        );
        assert_eq!(cmd.stdin.as_deref(), Some("secret"));
        // password never appears on the argv
        assert!(!cmd.args.iter().any(|a| a == "secret"));
    }

    #[test]
    fn blank_registry_is_omitted_docker_hub() {
        let host = MockHost::new().with_process_result(0, vec![]);
        let cfg = LoginConfig {
            registry: Some("  ".into()),
            username: "u".into(),
            password: "p".into(),
        };
        let _ = run(&Login, &ctx(&host), cfg);
        assert_eq!(host.recorded_commands()[0].args[0], "login");
        assert_eq!(host.recorded_commands()[0].args[1], "--username");
    }

    #[test]
    fn non_zero_exit_maps_to_login_failure() {
        let host = MockHost::new().with_process_result(1, vec![]);
        let cfg = LoginConfig {
            username: "u".into(),
            password: "p".into(),
            ..Default::default()
        };
        let w = run(&Login, &ctx(&host), cfg).into_wit();
        assert_eq!(
            w.error_message.as_deref(),
            Some("Failed to log in to Docker registry: Docker command failed with exit code 1")
        );
    }
}
