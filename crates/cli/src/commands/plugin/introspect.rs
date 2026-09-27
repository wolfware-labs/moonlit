use moonlit_engine::engine::Engine;
use moonlit_engine::logging::LogLevel;
use moonlit_engine::plugin::host::HostEventSink;
use moonlit_engine::plugin::middleware::MiddlewareInfo;
use moonlit_engine::plugin::{Plugin, PluginMetadata};
use std::sync::Arc;

struct SilentSink;
impl HostEventSink for SilentSink {
  fn log(&self, _step: &str, _level: LogLevel, _message: &str) {}
  fn progress(&self, _step: &str, _message: &str) {}
}

pub(super) async fn introspect(bytes: &[u8]) -> anyhow::Result<(PluginMetadata, Vec<MiddlewareInfo>)> {
  let engine = Engine::default()?;
  let cfg = PluginInstanceConfig {
    working_directory: std::env::temp_dir(),
    permissions: Permissions::deny(),
    config_view: serde_json::json!({}),
    env_snapshot: vec![],
  };
  let mut plugin_instance = Plugin::instantiate(&engine, bytes, cfg, Arc::new(SilentSink)).await?;
  let meta = plugin_instance.describe().await?;
  let mws = plugin_instance.list_middlewares().await?;
  Ok((meta, mws))
}
