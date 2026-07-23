//! moonlit plugin driven through the real engine host — offline: an empty
//! `modulePaths` hits the pinned guard and fails before any subprocess spawn.

use std::sync::Arc;

use moonlit_engine::config::model::{FilesystemAccess, Permissions};
use moonlit_engine::host::{
    HostEventSink, InstanceConfig, LogLevel, PluginInstance, ReleaseContext,
};

const FIXTURE: &[u8] = include_bytes!("fixtures/moonlit.wasm");

struct NullSink;
impl HostEventSink for NullSink {
    fn log(&self, _s: &str, _l: LogLevel, _m: &str) {}
    fn progress(&self, _s: &str, _m: &str) {}
}

fn cfg(workdir: &std::path::Path) -> InstanceConfig {
    InstanceConfig {
        working_directory: workdir.to_path_buf(),
        permissions: Permissions {
            network: vec![],
            exec: vec!["moonlit".to_string()],
            env: vec![],
            filesystem: FilesystemAccess::ReadWrite,
        },
        config_view: serde_json::json!({}),
        env_snapshot: vec![],
    }
}

fn ctx(workdir: &std::path::Path, step: &str) -> ReleaseContext {
    ReleaseContext {
        working_directory: workdir.to_str().unwrap().to_string(),
        step_name: step.to_string(),
    }
}

#[tokio::test]
async fn empty_module_paths_fails_before_spawn() {
    let dir = tempfile::tempdir().unwrap();
    let eng = moonlit_engine::host::test_engine();
    let mut p = PluginInstance::instantiate(&eng, FIXTURE, cfg(dir.path()), Arc::new(NullSink))
        .await
        .expect("instantiates");
    p.init(&serde_json::json!({})).await.expect("init ok");
    let r = p
        .execute(
            "run-modules",
            ctx(dir.path(), "modules"),
            &serde_json::json!({ "modulePaths": [] }),
        )
        .await
        .unwrap();
    assert!(!r.successful);
    assert_eq!(
        r.error_message.as_deref(),
        Some("No module paths provided for run-modules.")
    );
}
