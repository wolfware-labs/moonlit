//! docker plugin driven through the real engine host — tool-free contract paths only
//! (each fails before spawning `docker`), so no docker binary is needed and the run is
//! deterministic. Covers: login blank credentials, deploy swarm unsupported.

use std::sync::Arc;

use moonlit_engine::config::model::{FilesystemAccess, Permissions};
use moonlit_engine::host::{
    HostEventSink, InstanceConfig, LogLevel, PluginInstance, ReleaseContext,
};

const FIXTURE: &[u8] = include_bytes!("fixtures/docker.wasm");

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
            exec: vec!["docker".to_string()],
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
async fn login_blank_credentials_fails() {
    let dir = tempfile::tempdir().unwrap();
    let mut p = instance(dir.path(), serde_json::json!({})).await;
    let r = p
        .execute("login", ctx(dir.path(), "login"), &serde_json::json!({}))
        .await
        .unwrap();
    assert!(!r.successful);
    assert_eq!(
        r.error_message.as_deref(),
        Some("Docker login requires both username and password to be set.")
    );
}

#[tokio::test]
async fn deploy_service_is_swarm_unsupported() {
    let dir = tempfile::tempdir().unwrap();
    let mut p = instance(dir.path(), serde_json::json!({})).await;
    let r = p
        .execute(
            "deploy",
            ctx(dir.path(), "deploy"),
            &serde_json::json!({ "image": "img", "host": "ssh://h", "service": "web" }),
        )
        .await
        .unwrap();
    assert!(!r.successful);
    assert_eq!(
        r.error_message.as_deref(),
        Some("Swarm deploys are not supported yet.")
    );
}
