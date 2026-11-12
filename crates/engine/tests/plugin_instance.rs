use std::sync::Arc;

use moonlit_engine::engine::Engine;
use moonlit_engine::engine::config::EngineSettings;
use moonlit_engine::logging::LogLevel;
use moonlit_engine::pipeline::config::Permissions;
use moonlit_engine::plugin::host::{HostEventSink, ReleaseContext};
use moonlit_engine::plugin::permissions::PermissionKind;
use moonlit_engine::plugin::{Plugin, PluginError, PluginInstance, PluginInstanceConfig};

const FIXTURE: &[u8] = include_bytes!("fixtures/test_plugin.wasm");

struct NullSink;

impl HostEventSink for NullSink {
  fn log(&self, _step: &str, _level: LogLevel, _message: &str) {}
  fn progress(&self, _step: &str, _message: &str) {}
}

fn engine() -> Engine {
  let cache = tempfile::tempdir().unwrap();
  Engine::new(EngineSettings {
    cache_dir: Some(cache.path().to_path_buf()),
    ..EngineSettings::default()
  })
  .unwrap()
}

fn config() -> PluginInstanceConfig {
  PluginInstanceConfig {
    working_directory: std::env::temp_dir(),
    permissions: Permissions::full_trust(),
    config_view: serde_json::json!({}),
    env_snapshot: vec![],
  }
}

async fn trapped_instance() -> PluginInstance {
  let mut instance = Plugin::instantiate(&engine(), FIXTURE, config(), Arc::new(NullSink))
    .await
    .unwrap();
  let context = ReleaseContext {
    working_directory: "/work".to_string(),
    step_name: "boom".to_string(),
  };
  let trap = instance.execute("boom", context, &serde_json::json!({})).await;
  assert!(matches!(trap, Err(PluginError::Trap { .. })), "{trap:?}");
  instance
}

#[tokio::test]
async fn describe_after_a_trap_reports_a_trap() {
  let mut instance = trapped_instance().await;
  let result = instance.describe().await;
  assert!(
    matches!(&result, Err(PluginError::Trap { op, .. }) if op == "describe"),
    "{result:?}"
  );
}

#[tokio::test]
async fn list_middlewares_after_a_trap_reports_a_trap() {
  let mut instance = trapped_instance().await;
  let result = instance.list_middlewares().await;
  assert!(
    matches!(&result, Err(PluginError::Trap { op, .. }) if op == "list-middlewares"),
    "{result:?}"
  );
}

#[tokio::test]
async fn init_after_a_trap_reports_the_trap() {
  let mut instance = trapped_instance().await;
  let result = instance.init(&serde_json::json!({})).await;
  assert!(
    matches!(&result, Err(msg) if msg.starts_with("plugin trapped during init")),
    "{result:?}"
  );
}

#[tokio::test]
async fn bytes_that_are_not_a_component_fail_to_compile() {
  let result = Plugin::instantiate(&engine(), b"not wasm", config(), Arc::new(NullSink)).await;
  assert!(matches!(result, Err(PluginError::Compile(_))));
}

#[tokio::test]
async fn describe_returns_declared_permissions() {
  let mut instance = Plugin::instantiate(&engine(), FIXTURE, config(), Arc::new(NullSink))
    .await
    .unwrap();
  let meta = instance.describe().await.unwrap();
  assert_eq!(meta.permissions.len(), 2);
  assert_eq!(meta.permissions[0].kind, PermissionKind::Network);
  assert_eq!(meta.permissions[0].values, vec!["api.example.com"]);
  assert!(!meta.permissions[0].optional);
  assert_eq!(meta.permissions[1].kind, PermissionKind::Exec);
  assert!(meta.permissions[1].optional);
}

async fn describe_declaring_exec(value: &str) -> Result<moonlit_engine::plugin::PluginMetadata, PluginError> {
  let cfg = PluginInstanceConfig {
    env_snapshot: vec![("TEST_PLUGIN_EXTRA_EXEC".to_string(), value.to_string())],
    ..config()
  };
  let mut instance = Plugin::instantiate(&engine(), FIXTURE, cfg, Arc::new(NullSink))
    .await
    .unwrap();
  instance.describe().await
}

#[tokio::test]
async fn describe_rejects_a_declared_value_with_unsupported_glob_syntax() {
  let result = describe_declaring_exec("{git,sh}").await;
  let Err(PluginError::Load(message)) = &result else {
    panic!("expected a load error, got {result:?}");
  };
  assert!(
    message.contains("`{git,sh}`") && message.contains("only `*` and `?` wildcards"),
    "{message}"
  );
}

#[tokio::test]
async fn describe_accepts_a_declared_wildcard_value() {
  let meta = describe_declaring_exec("cargo-*").await.unwrap();
  assert!(
    meta
      .permissions
      .iter()
      .any(|p| p.kind == PermissionKind::Exec && p.values == ["cargo-*"]),
    "{:?}",
    meta.permissions
  );
}
