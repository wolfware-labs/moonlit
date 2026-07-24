//! Shared component introspection: instantiate a component with zero capability grants and
//! read its metadata + middleware list. Used by `plugin inspect` and `plugin publish`.

use std::sync::Arc;

use moonlit_engine::config::model::{FilesystemAccess, Permissions};
use moonlit_engine::host::{
    HostEventSink, InstanceConfig, LogLevel, MiddlewareInfo, PluginInstance, PluginMetadata,
    test_engine,
};

/// Introspection never needs guest logs; discard them.
struct SilentSink;
impl HostEventSink for SilentSink {
    fn log(&self, _step: &str, _level: LogLevel, _message: &str) {}
    fn progress(&self, _step: &str, _message: &str) {}
}

/// Zero grants: `describe`/`list-middlewares` require no capabilities.
fn no_permissions() -> Permissions {
    Permissions {
        network: vec![],
        exec: vec![],
        env: vec![],
        filesystem: FilesystemAccess::None,
    }
}

/// Instantiate `bytes` and read its metadata + middleware list. `describe` (not `init`) is
/// used so a plugin with required config still introspects cleanly.
pub(super) async fn introspect(
    bytes: &[u8],
) -> Result<(PluginMetadata, Vec<MiddlewareInfo>), String> {
    let engine = test_engine();
    let cfg = InstanceConfig {
        working_directory: std::env::temp_dir(),
        permissions: no_permissions(),
        config_view: serde_json::json!({}),
        env_snapshot: vec![],
    };
    let mut inst = PluginInstance::instantiate(&engine, bytes, cfg, Arc::new(SilentSink))
        .await
        .map_err(|e| e.to_string())?;
    let meta = inst.describe().await.map_err(|e| e.to_string())?;
    let mws = inst.list_middlewares().await.map_err(|e| e.to_string())?;
    Ok((meta, mws))
}
