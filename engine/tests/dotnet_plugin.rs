//! dotnet plugin driven through the real engine host — tool-free contract paths only
//! (each fails before spawning `dotnet`), so no .NET SDK is needed and the run is
//! deterministic. Covers: build missing-project, push missing-source, push missing-key.

use std::sync::Arc;

use moonlit_engine::config::model::{FilesystemAccess, Permissions};
use moonlit_engine::host::{
    HostEventSink, InstanceConfig, LogLevel, PluginInstance, ReleaseContext,
};

const FIXTURE: &[u8] = include_bytes!("fixtures/dotnet.wasm");

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
            exec: vec!["dotnet".to_string()],
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
async fn build_missing_project_fails() {
    let dir = tempfile::tempdir().unwrap();
    let mut p = instance(dir.path(), serde_json::json!({})).await;
    let r = p
        .execute(
            "build",
            ctx(dir.path(), "build"),
            &serde_json::json!({ "project": "missing.csproj", "version": "1.0.0" }),
        )
        .await
        .unwrap();
    assert!(!r.successful);
    assert_eq!(
        r.error_message.as_deref(),
        Some("Project file not found at path: missing.csproj")
    );
}

#[tokio::test]
async fn push_missing_source_fails() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("App.nupkg"), b"pkg").unwrap();
    // Blank plugin source + no local source -> the not-specified failure, before any spawn.
    let mut p = instance(dir.path(), serde_json::json!({ "nugetSource": "" })).await;
    let r = p
        .execute(
            "push",
            ctx(dir.path(), "push"),
            &serde_json::json!({ "package": "App.nupkg" }),
        )
        .await
        .unwrap();
    assert!(!r.successful);
    assert_eq!(
        r.error_message.as_deref(),
        Some("NuGet source is not specified in both global and local configuration.")
    );
}

#[tokio::test]
async fn push_missing_key_fails() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("App.nupkg"), b"pkg").unwrap();
    // Source present (plugin default), but no API key anywhere.
    let mut p = instance(
        dir.path(),
        serde_json::json!({ "nugetSource": "https://feed" }),
    )
    .await;
    let r = p
        .execute(
            "push",
            ctx(dir.path(), "push"),
            &serde_json::json!({ "package": "App.nupkg" }),
        )
        .await
        .unwrap();
    assert!(!r.successful);
    assert_eq!(
        r.error_message.as_deref(),
        Some("NuGet API key is not specified in both global and local configuration.")
    );
}
