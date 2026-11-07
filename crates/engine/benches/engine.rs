use std::hint::black_box;
use std::sync::Arc;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use moonlit_engine::engine::Engine;
use moonlit_engine::engine::config::EngineSettings;
use moonlit_engine::logging::LogLevel;
use moonlit_engine::pipeline::config::{FilesystemAccess, Permissions, parse_config};
use moonlit_engine::plugin::host::{HostEventSink, ReleaseContext};
use moonlit_engine::plugin::permissions::{DeclaredPermission, PermissionKind, uncovered_required};
use moonlit_engine::plugin::{Plugin, PluginInstanceConfig};
use tokio::runtime::Runtime;

const FIXTURE: &[u8] = include_bytes!("../tests/fixtures/test_plugin.wasm");

const RELEASE_YML: &str = include_str!("fixtures/release.yml");

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

fn declaration(kind: PermissionKind, values: &[&str], optional: bool) -> DeclaredPermission {
  DeclaredPermission {
    kind,
    values: values.iter().map(|v| (*v).to_string()).collect(),
    reason: "benchmark".to_string(),
    optional,
  }
}

fn check_grants(c: &mut Criterion) {
  let declared = vec![
    declaration(PermissionKind::Network, &["api.github.com", "uploads.github.com"], false),
    declaration(PermissionKind::Exec, &["git", "gh"], true),
    declaration(PermissionKind::Env, &["GITHUB_TOKEN", "GITHUB_REF", "CI"], false),
    declaration(PermissionKind::FilesystemRead, &[], false),
    declaration(PermissionKind::FilesystemWrite, &[], true),
  ];
  let granted = Permissions {
    network: vec!["*.github.com".to_string(), "ghcr.io".to_string()],
    exec: vec!["git".to_string()],
    env: vec!["GITHUB_*".to_string(), "CI".to_string(), "RUNNER_*".to_string()],
    filesystem: FilesystemAccess::ReadOnly,
  };
  c.bench_function("check_grants", |b| {
    b.iter(|| uncovered_required(black_box(&declared), black_box(&granted)).len());
  });
}

criterion_group!(benches, parse_release_config, load_plugin, execute_middleware, check_grants);
criterion_main!(benches);
