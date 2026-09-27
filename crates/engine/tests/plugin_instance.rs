use std::sync::Arc;

use moonlit_engine::engine::Engine;
use moonlit_engine::engine::config::EngineSettings;
use moonlit_engine::logging::LogLevel;
use moonlit_engine::pipeline::config::Permissions;
use moonlit_engine::plugin::host::{HostEventSink, ReleaseContext};
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
