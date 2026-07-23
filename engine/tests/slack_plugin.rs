//! slack plugin driven through the real engine host — network-free: init
//! validation and a blank-channel failure that returns before any HTTP call.

use std::sync::Arc;

use moonlit_engine::config::model::{FilesystemAccess, Permissions};
use moonlit_engine::host::{
    HostEventSink, InstanceConfig, LogLevel, PluginInstance, ReleaseContext,
};

const FIXTURE: &[u8] = include_bytes!("fixtures/slack.wasm");

struct NullSink;
impl HostEventSink for NullSink {
    fn log(&self, _s: &str, _l: LogLevel, _m: &str) {}
    fn progress(&self, _s: &str, _m: &str) {}
}

fn cfg(workdir: &std::path::Path) -> InstanceConfig {
    InstanceConfig {
        working_directory: workdir.to_path_buf(),
        permissions: Permissions {
            network: vec!["slack.com".to_string()],
            exec: vec![],
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
async fn blank_token_fails_init_with_exact_message() {
    let dir = tempfile::tempdir().unwrap();
    let eng = moonlit_engine::host::test_engine();
    let mut p = PluginInstance::instantiate(&eng, FIXTURE, cfg(dir.path()), Arc::new(NullSink))
        .await
        .expect("instantiates");
    match p.init(&serde_json::json!({ "token": "" })).await {
        Ok(_) => panic!("blank token must fail init"),
        Err(e) => assert_eq!(e, "Slack API token is required."),
    }
}

#[tokio::test]
async fn blank_channel_fails_before_request() {
    let dir = tempfile::tempdir().unwrap();
    let eng = moonlit_engine::host::test_engine();
    let mut p = PluginInstance::instantiate(&eng, FIXTURE, cfg(dir.path()), Arc::new(NullSink))
        .await
        .expect("instantiates");
    p.init(&serde_json::json!({ "token": "xoxb-dummy" }))
        .await
        .expect("init ok");
    let r = p
        .execute(
            "send-notification",
            ctx(dir.path(), "notify"),
            &serde_json::json!({ "message": "hi" }),
        )
        .await
        .unwrap();
    assert!(!r.successful);
    assert_eq!(
        r.error_message.as_deref(),
        Some("No Slack channel provided for notification.")
    );
}
