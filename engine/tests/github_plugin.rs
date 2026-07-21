//! github plugin driven through the real engine host — network-free: init
//! validation, empty related-items, and a real write-variables file append.

use std::sync::Arc;

use moonlit_engine::config::model::{FilesystemAccess, Permissions};
use moonlit_engine::host::{
    HostEventSink, InstanceConfig, LogLevel, PluginInstance, ReleaseContext,
};

const FIXTURE: &[u8] = include_bytes!("fixtures/github.wasm");

struct NullSink;
impl HostEventSink for NullSink {
    fn log(&self, _s: &str, _l: LogLevel, _m: &str) {}
    fn progress(&self, _s: &str, _m: &str) {}
}

fn cfg(workdir: &std::path::Path, env: Vec<(String, String)>) -> InstanceConfig {
    InstanceConfig {
        working_directory: workdir.to_path_buf(),
        permissions: Permissions {
            network: vec!["api.github.com".to_string()],
            exec: vec!["git".to_string(), "sh".to_string()],
            env: vec!["*".to_string()],
            filesystem: FilesystemAccess::ReadWrite,
        },
        config_view: serde_json::json!({}),
        env_snapshot: env,
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
    let mut p =
        PluginInstance::instantiate(&eng, FIXTURE, cfg(dir.path(), vec![]), Arc::new(NullSink))
            .await
            .expect("instantiates");
    match p.init(&serde_json::json!({ "token": "" })).await {
        Ok(_) => panic!("blank token must fail init"),
        Err(e) => assert_eq!(e, "GitHub token is not configured."),
    }
}

#[tokio::test]
async fn empty_related_items_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let eng = moonlit_engine::host::test_engine();
    let mut p =
        PluginInstance::instantiate(&eng, FIXTURE, cfg(dir.path(), vec![]), Arc::new(NullSink))
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
async fn write_variables_appends_to_github_output_file() {
    let dir = tempfile::tempdir().unwrap();
    let out_file = dir.path().join("gh_output");
    std::fs::write(&out_file, "").unwrap();
    let env = vec![(
        "GITHUB_OUTPUT".to_string(),
        out_file.to_str().unwrap().to_string(),
    )];
    let eng = moonlit_engine::host::test_engine();
    let mut p =
        PluginInstance::instantiate(&eng, FIXTURE, cfg(dir.path(), env), Arc::new(NullSink))
            .await
            .expect("instantiates");
    p.init(&serde_json::json!({ "token": "dummy" }))
        .await
        .expect("init ok");
    let r = p
        .execute(
            "write-variables",
            ctx(dir.path(), "wv"),
            &serde_json::json!({ "output": { "version": "1.2.3", "notes": "a\nb" } }),
        )
        .await
        .unwrap();
    assert!(
        r.successful,
        "write-variables failed: {:?}",
        r.error_message
    );
    let written = std::fs::read_to_string(&out_file).unwrap();
    assert!(written.contains("version=1.2.3\n"), "got: {written}");
    assert!(
        written.contains("notes<<EOF\na\nb\nEOF\n"),
        "got: {written}"
    );
}
