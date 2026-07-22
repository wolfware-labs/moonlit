//! gitlab plugin driven through the real engine host — network-free: init
//! validation, empty related-items, and a real write-variables dotenv append.

use std::sync::Arc;

use moonlit_engine::config::model::{FilesystemAccess, Permissions};
use moonlit_engine::host::{
    HostEventSink, InstanceConfig, LogLevel, PluginInstance, ReleaseContext,
};

const FIXTURE: &[u8] = include_bytes!("fixtures/gitlab.wasm");

struct NullSink;
impl HostEventSink for NullSink {
    fn log(&self, _s: &str, _l: LogLevel, _m: &str) {}
    fn progress(&self, _s: &str, _m: &str) {}
}

fn cfg(workdir: &std::path::Path) -> InstanceConfig {
    InstanceConfig {
        working_directory: workdir.to_path_buf(),
        permissions: Permissions {
            network: vec!["gitlab.com".to_string()],
            exec: vec!["git".to_string()],
            env: vec!["*".to_string()],
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
async fn blank_token_fails_init_with_exact_message() {
    let dir = tempfile::tempdir().unwrap();
    let eng = moonlit_engine::host::test_engine();
    let mut p = PluginInstance::instantiate(&eng, FIXTURE, cfg(dir.path()), Arc::new(NullSink))
        .await
        .expect("instantiates");
    match p.init(&serde_json::json!({ "token": "" })).await {
        Ok(_) => panic!("blank token must fail init"),
        Err(e) => assert_eq!(e, "GitLab token is not configured."),
    }
}

#[tokio::test]
async fn empty_related_items_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let eng = moonlit_engine::host::test_engine();
    let mut p = PluginInstance::instantiate(&eng, FIXTURE, cfg(dir.path()), Arc::new(NullSink))
        .await
        .expect("instantiates");
    p.init(&serde_json::json!({ "token": "dummy" }))
        .await
        .expect("init ok");
    let r = p
        .execute(
            "related-items",
            ctx(dir.path(), "ri"),
            &serde_json::json!({ "commits": [] }),
        )
        .await
        .unwrap();
    assert!(
        r.successful,
        "empty commits should succeed: {:?}",
        r.error_message
    );
}

#[tokio::test]
async fn write_variables_appends_dotenv_file() {
    let dir = tempfile::tempdir().unwrap();
    let eng = moonlit_engine::host::test_engine();
    let mut p = PluginInstance::instantiate(&eng, FIXTURE, cfg(dir.path()), Arc::new(NullSink))
        .await
        .expect("instantiates");
    p.init(&serde_json::json!({ "token": "dummy" }))
        .await
        .expect("init ok");
    let r = p
        .execute(
            "write-variables",
            ctx(dir.path(), "wv"),
            &serde_json::json!({ "output": { "VERSION": "1.2.3", "NOTES": "a\nb" } }),
        )
        .await
        .unwrap();
    assert!(
        r.successful,
        "write-variables failed: {:?}",
        r.error_message
    );
    // Proves the wasm relative-path write landed in the preopened working dir.
    let written = std::fs::read_to_string(dir.path().join("moonlit.env")).unwrap();
    assert!(written.contains("VERSION=1.2.3\n"), "got: {written}");
    assert!(written.contains("NOTES=a\\nb\n"), "got: {written}");
}
