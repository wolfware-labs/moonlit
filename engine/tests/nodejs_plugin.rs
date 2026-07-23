//! nodejs plugin driven through the real engine host — tool-free contract paths only
//! (each fails before spawning `npm`), so no npm/Node is needed and the run is
//! deterministic. Covers: run-script + build missing package.json, push missing tarball.

use std::sync::Arc;

use moonlit_engine::config::model::{FilesystemAccess, Permissions};
use moonlit_engine::host::{
    HostEventSink, InstanceConfig, LogLevel, PluginInstance, ReleaseContext,
};

const FIXTURE: &[u8] = include_bytes!("fixtures/nodejs.wasm");

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
            exec: vec!["npm".to_string(), "node".to_string()],
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

async fn instance(dir: &std::path::Path, plugin_cfg: serde_json::Value) -> PluginInstance {
    let eng = moonlit_engine::host::test_engine();
    let mut p = PluginInstance::instantiate(&eng, FIXTURE, cfg(dir), Arc::new(NullSink))
        .await
        .expect("instantiates");
    p.init(&plugin_cfg).await.expect("init ok");
    p
}

#[tokio::test]
async fn run_script_missing_package_json_fails() {
    let dir = tempfile::tempdir().unwrap();
    let mut p = instance(dir.path(), serde_json::json!({})).await;
    let r = p
        .execute(
            "run-script",
            ctx(dir.path(), "run-script"),
            &serde_json::json!({ "script": "build" }),
        )
        .await
        .unwrap();
    assert!(!r.successful);
    assert!(
        r.error_message
            .as_deref()
            .unwrap()
            .starts_with("package.json not found in directory:")
    );
}

#[tokio::test]
async fn build_missing_package_json_fails() {
    let dir = tempfile::tempdir().unwrap();
    let mut p = instance(dir.path(), serde_json::json!({})).await;
    let r = p
        .execute(
            "build",
            ctx(dir.path(), "build"),
            &serde_json::json!({ "command": "build" }),
        )
        .await
        .unwrap();
    assert!(!r.successful);
    assert!(
        r.error_message
            .as_deref()
            .unwrap()
            .starts_with("package.json not found in directory:")
    );
}

#[tokio::test]
async fn push_missing_tarball_fails() {
    let dir = tempfile::tempdir().unwrap();
    let mut p = instance(dir.path(), serde_json::json!({ "token": "T" })).await;
    let r = p
        .execute(
            "push",
            ctx(dir.path(), "push"),
            &serde_json::json!({ "package": "missing.tgz" }),
        )
        .await
        .unwrap();
    assert!(!r.successful);
    assert!(
        r.error_message
            .as_deref()
            .unwrap()
            .starts_with("Package tarball not found at path:")
    );
}
