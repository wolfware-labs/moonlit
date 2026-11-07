use std::hint::black_box;
use std::sync::Arc;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use moonlit_engine::engine::Engine;
use moonlit_engine::engine::config::EngineSettings;
use moonlit_engine::logging::LogLevel;
use moonlit_engine::pipeline::config::{Permissions, parse_config};
use moonlit_engine::plugin::host::{HostEventSink, ReleaseContext};
use moonlit_engine::plugin::{Plugin, PluginInstanceConfig};
use tokio::runtime::Runtime;

const FIXTURE: &[u8] = include_bytes!("../tests/fixtures/test_plugin.wasm");

const RELEASE_YML: &str = r#"name: acme-service

arguments:
  version: "1.4.0"
  channel: stable

variables:
  registry: ghcr.io/acme
  region: eu-west-1

plugins:
  - name: git
    url: oci://registry.moonlit.rs/wolfware/git:1.3.0
    permissions:
      exec: ["git"]
      env: ["GIT_*"]
      filesystem: read-write
  - name: changelog
    url: oci://registry.moonlit.rs/wolfware/changelog:2.1.0
    config:
      format: keep-a-changelog
      sections: [added, changed, fixed, removed]
    permissions:
      filesystem: read-write
  - name: cargo
    url: oci://registry.moonlit.rs/wolfware/cargo:1.0.4
    permissions:
      exec: ["cargo", "rustc"]
      env: ["CARGO_*", "RUST*"]
      filesystem: read-write
  - name: docker
    url: oci://registry.moonlit.rs/wolfware/docker:3.2.0
    config:
      registry: $(vars:registry)
      platforms: [linux/amd64, linux/arm64]
    permissions:
      network: ["ghcr.io"]
      exec: ["docker"]
      env: ["DOCKER_*"]
      filesystem: read-only
  - name: github
    url: oci://registry.moonlit.rs/wolfware/github:2.0.0
    config:
      owner: acme
      repo: service
    permissions:
      network: ["api.github.com", "uploads.github.com"]
      env: ["GITHUB_*"]
      filesystem: read-only

stages:
  prepare:
    - name: tags
      run: git.list-tags
      config:
        pattern: "v*"
    - name: commits
      run: git.commits-since
      config:
        since: $(output:tags)
    - name: bump
      run: changelog.next-version
      config:
        current: $(args:version)
        commits: $(output:commits)
  build:
    - name: compile
      run: cargo.build
      config:
        profile: release
        features: [tls, metrics]
        locked: true
    - name: test
      run: cargo.test
      continueOnError: false
      config:
        workspace: true
    - name: image
      run: docker.build
      condition: "args.channel == 'stable'"
      config:
        tag: $(vars:registry)/service:v$(output:bump)
        context: .
        labels:
          org.opencontainers.image.version: $(output:bump)
          org.opencontainers.image.source: https://github.com/acme/service
  release:
    - name: notes
      run: changelog.render
      config:
        version: $(output:bump)
        commits: $(output:commits)
    - name: tag
      run: git.create-tag
      haltIf: "output.bump == ''"
      config:
        name: v$(output:bump)
        message: Release v$(output:bump)
    - name: push-image
      run: docker.push
      continueOnError: true
      config:
        image: $(output:image)
    - name: publish
      run: github.create-release
      config:
        tag: v$(output:bump)
        name: v$(output:bump)
        body: $(output:notes)
        draft: false
        assets: [target/release/service]
"#;

struct NullSink;

impl HostEventSink for NullSink {
  fn log(&self, _step: &str, _level: LogLevel, _message: &str) {}
  fn progress(&self, _step: &str, _message: &str) {}
}

fn instance_config() -> PluginInstanceConfig {
  PluginInstanceConfig {
    working_directory: std::env::temp_dir(),
    permissions: Permissions::full_trust(),
    config_view: serde_json::json!({ "plugin": { "name": "test-plugin" } }),
    env_snapshot: vec![],
  }
}

fn engine() -> (Engine, tempfile::TempDir) {
  let cache = tempfile::tempdir().expect("cache dir");
  let engine = Engine::new(EngineSettings {
    cache_dir: Some(cache.path().to_path_buf()),
    ..EngineSettings::default()
  })
  .expect("engine");
  (engine, cache)
}

fn parse_release_config(c: &mut Criterion) {
  c.bench_function("parse_release_config", |b| {
    b.iter(|| parse_config(black_box(RELEASE_YML), "release.yml").expect("valid release.yml"));
  });
}

fn load_plugin(c: &mut Criterion) {
  let rt = Runtime::new().expect("tokio runtime");
  let (engine, _cache) = engine();
  let sink: Arc<dyn HostEventSink> = Arc::new(NullSink);
  let plugin_config = serde_json::json!({});
  c.bench_function("load_plugin", |b| {
    b.iter_batched(
      instance_config,
      |cfg| {
        rt.block_on(async {
          let mut instance = Plugin::instantiate(&engine, black_box(FIXTURE), cfg, sink.clone())
            .await
            .expect("instantiate");
          instance.init(&plugin_config).await.expect("init");
          let middlewares = instance.list_middlewares().await.expect("list middlewares");
          (instance, middlewares)
        })
      },
      BatchSize::PerIteration,
    );
  });
}

fn execute_middleware(c: &mut Criterion) {
  let rt = Runtime::new().expect("tokio runtime");
  let (engine, _cache) = engine();
  let mut instance = rt
    .block_on(Plugin::instantiate(&engine, FIXTURE, instance_config(), Arc::new(NullSink)))
    .expect("instantiate");
  rt.block_on(instance.init(&serde_json::json!({}))).expect("init");
  let step_config = serde_json::json!({ "name": "acme-service", "version": "1.4.0" });
  let context = ReleaseContext {
    working_directory: "/work".to_string(),
    step_name: "compile".to_string(),
  };
  c.bench_function("execute_middleware", |b| {
    b.iter_batched(
      || context.clone(),
      |ctx| {
        rt.block_on(instance.execute("log-and-output", ctx, black_box(&step_config)))
          .expect("execute")
      },
      BatchSize::SmallInput,
    );
  });
}

criterion_group!(benches, parse_release_config, load_plugin, execute_middleware);
criterion_main!(benches);
