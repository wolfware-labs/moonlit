//! `docker build` (buildx) — build an image, then push (default) or load it.
//! Argument order is fixed per MVP_SPEC §11.7.

use crate::docker::{docker, fail};
use crate::state::DockerShared;
use moonlit_plugin_sdk::prelude::*;
use moonlit_plugin_sdk::process::LineHandler;
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BuildAndPushConfig {
    pub builder: Option<String>,
    pub tags: Vec<String>,
    pub file: Option<String>,
    pub context: String,
    pub push: bool,
    pub build_args: Vec<String>,
    pub labels: BTreeMap<String, String>,
    pub platforms: Vec<String>,
    pub no_cache: bool,
    pub pull: bool,
    pub cache_from: Vec<String>,
    pub cache_to: Vec<String>,
}

impl Default for BuildAndPushConfig {
    fn default() -> Self {
        Self {
            builder: None,
            tags: Vec::new(),
            file: None,
            context: ".".to_string(),
            push: true,
            build_args: Vec::new(),
            labels: BTreeMap::new(),
            platforms: Vec::new(),
            no_cache: false,
            pull: false,
            cache_from: Vec::new(),
            cache_to: Vec::new(),
        }
    }
}

#[derive(Default)]
pub struct BuildAndPush;

/// Builder resolution: explicit config → shared state → env var. First non-blank wins.
fn resolve_builder(ctx: &Context, cfg: &BuildAndPushConfig) -> Option<String> {
    if let Some(b) = cfg.builder.as_deref().filter(|b| !b.trim().is_empty()) {
        return Some(b.to_string());
    }
    if let Some(b) = ctx
        .state::<DockerShared>()
        .builder
        .get()
        .filter(|b| !b.trim().is_empty())
    {
        return Some(b);
    }
    ctx.env()
        .var("MOONLIT_DOCKER_BUILDX_BUILDER")
        .filter(|b| !b.trim().is_empty())
}

impl Middleware for BuildAndPush {
    const NAME: &'static str = "build-and-push";
    const DESCRIPTION: &'static str = "build a docker image and push or load it";
    type Config = BuildAndPushConfig;

    fn execute(&self, ctx: &Context, cfg: BuildAndPushConfig) -> MiddlewareResult {
        let mut c = docker(ctx).arg("build");
        if let Some(b) = resolve_builder(ctx, &cfg) {
            c = c.arg("--builder").arg(b);
        }
        for t in &cfg.tags {
            c = c.arg("--tag").arg(t);
        }
        if let Some(f) = cfg.file.as_deref().filter(|f| !f.trim().is_empty()) {
            c = c.arg("--file").arg(f);
        }
        for a in &cfg.build_args {
            c = c.arg("--build-arg").arg(a);
        }
        for (k, v) in &cfg.labels {
            c = c.arg("--label").arg(format!("{k}={v}"));
        }
        if !cfg.platforms.is_empty() {
            c = c.arg("--platform").arg(cfg.platforms.join(","));
        }
        if cfg.no_cache {
            c = c.arg("--no-cache");
        }
        for cf in &cfg.cache_from {
            c = c.arg("--cache-from").arg(cf);
        }
        for ct in &cfg.cache_to {
            c = c.arg("--cache-to").arg(ct);
        }
        if cfg.pull {
            c = c.arg("--pull");
        }
        c = c.arg(if cfg.push { "--push" } else { "--load" });
        c = c.arg(&cfg.context);
        match c.stream(LineHandler::severity()) {
            Ok(o) if o.success() => MiddlewareResult::success(),
            Ok(o) => fail("build and push image", o.exit_code),
            Err(e) => MiddlewareResult::failure(format!("Failed to build and push image: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_plugin_sdk::testing::{run, MockHost};

    fn ctx<'a>(host: &'a MockHost, shared: &'a DockerShared) -> Context<'a> {
        Context::new(host, "/wd".into(), "build-and-push".into()).with_state(shared)
    }

    #[test]
    fn full_argv_in_spec_order_push_default() {
        let host = MockHost::new().with_process_result(0, vec![]);
        let shared = DockerShared::default();
        let mut labels = BTreeMap::new();
        labels.insert("z".into(), "1".into());
        labels.insert("a".into(), "2".into()); // BTreeMap sorts -> a then z
        let cfg = BuildAndPushConfig {
            builder: Some("b1".into()),
            tags: vec!["img:1".into(), "img:latest".into()],
            file: Some("Dockerfile".into()),
            context: "app".into(),
            push: true,
            build_args: vec!["K=v".into()],
            labels,
            platforms: vec!["linux/amd64".into(), "linux/arm64".into()],
            no_cache: true,
            pull: true,
            cache_from: vec!["type=registry,ref=r".into()],
            cache_to: vec!["type=inline".into()],
        };
        assert!(run(&BuildAndPush, &ctx(&host, &shared), cfg).is_success());
        assert_eq!(
            host.recorded_commands()[0].args,
            vec![
                "build",
                "--builder",
                "b1",
                "--tag",
                "img:1",
                "--tag",
                "img:latest",
                "--file",
                "Dockerfile",
                "--build-arg",
                "K=v",
                "--label",
                "a=2",
                "--label",
                "z=1",
                "--platform",
                "linux/amd64,linux/arm64",
                "--no-cache",
                "--cache-from",
                "type=registry,ref=r",
                "--cache-to",
                "type=inline",
                "--pull",
                "--push",
                "app",
            ]
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>()
        );
    }

    #[test]
    fn push_false_uses_load_and_default_context_dot() {
        let host = MockHost::new().with_process_result(0, vec![]);
        let shared = DockerShared::default();
        let cfg = BuildAndPushConfig {
            push: false,
            ..Default::default()
        };
        let _ = run(&BuildAndPush, &ctx(&host, &shared), cfg);
        let args = &host.recorded_commands()[0].args;
        assert_eq!(
            args,
            &vec!["build".to_string(), "--load".to_string(), ".".to_string()]
        );
    }

    #[test]
    fn builder_falls_back_to_shared_state() {
        let host = MockHost::new().with_process_result(0, vec![]);
        let shared = DockerShared::default();
        shared.builder.set(Some("from-state".into()));
        let _ = run(
            &BuildAndPush,
            &ctx(&host, &shared),
            BuildAndPushConfig::default(),
        );
        let args = &host.recorded_commands()[0].args;
        let i = args.iter().position(|a| a == "--builder").unwrap();
        assert_eq!(args[i + 1], "from-state");
    }

    #[test]
    fn builder_falls_back_to_env_when_no_config_or_state() {
        let host = MockHost::new()
            .with_env("MOONLIT_DOCKER_BUILDX_BUILDER", "from-env")
            .with_process_result(0, vec![]);
        let shared = DockerShared::default();
        let _ = run(
            &BuildAndPush,
            &ctx(&host, &shared),
            BuildAndPushConfig::default(),
        );
        let args = &host.recorded_commands()[0].args;
        let i = args.iter().position(|a| a == "--builder").unwrap();
        assert_eq!(args[i + 1], "from-env");
    }

    #[test]
    fn no_builder_anywhere_omits_flag() {
        let host = MockHost::new().with_process_result(0, vec![]);
        let shared = DockerShared::default();
        let _ = run(
            &BuildAndPush,
            &ctx(&host, &shared),
            BuildAndPushConfig::default(),
        );
        assert!(!host.recorded_commands()[0]
            .args
            .iter()
            .any(|a| a == "--builder"));
    }

    #[test]
    fn non_zero_exit_maps_to_failure() {
        let host = MockHost::new().with_process_result(1, vec![]);
        let shared = DockerShared::default();
        let w = run(
            &BuildAndPush,
            &ctx(&host, &shared),
            BuildAndPushConfig::default(),
        )
        .into_wit();
        assert_eq!(
            w.error_message.as_deref(),
            Some("Failed to build and push image: Docker command failed with exit code 1")
        );
    }
}
