//! semantic-release plugin driven through the real engine host — fully offline
//! (zero permissions): analyze -> calculate-version -> generate-changelog.

use std::sync::Arc;

use moonlit_engine::config::model::{FilesystemAccess, Permissions};
use moonlit_engine::host::{
    HostEventSink, InstanceConfig, LogLevel, PluginInstance, ReleaseContext,
};

const FIXTURE: &[u8] = include_bytes!("fixtures/semantic-release.wasm");

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
            exec: vec![],
            env: vec![],
            filesystem: FilesystemAccess::None,
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

async fn instance(dir: &std::path::Path) -> PluginInstance {
    let eng = moonlit_engine::host::test_engine();
    let mut p = PluginInstance::instantiate(&eng, FIXTURE, cfg(dir), Arc::new(NullSink))
        .await
        .expect("instantiates");
    // No plugin config for this plugin; init with an empty object.
    p.init(&serde_json::json!({})).await.expect("init ok");
    p
}

#[tokio::test]
async fn analyze_then_calculate_then_changelog() {
    let dir = tempfile::tempdir().unwrap();
    let mut p = instance(dir.path()).await;

    // analyze: two raw commits -> conventional commits stored in shared state
    let a = p
        .execute(
            "analyze",
            ctx(dir.path(), "analyze"),
            &serde_json::json!({
                "commits": [
                    { "sha": "abc1234def", "date": "2026-01-01T00:00:00Z", "message": "feat: add thing" },
                    { "sha": "def5678abc", "date": "2026-02-01T00:00:00Z", "message": "fix: patch thing" }
                ]
            }),
        )
        .await
        .unwrap();
    assert!(a.successful, "analyze failed: {:?}", a.error_message);
    let a_out: std::collections::HashMap<String, serde_json::Value> =
        a.output.into_iter().collect();
    assert_eq!(a_out["commitCount"], serde_json::json!(2));

    // calculate-version: falls back to shared commits, feat -> minor bump, newest sha metadata
    let v = p
        .execute(
            "calculate-version",
            ctx(dir.path(), "version"),
            &serde_json::json!({ "baseVersion": "1.2.3" }),
        )
        .await
        .unwrap();
    assert!(v.successful, "calculate-version failed: {:?}", v.error_message);
    let v_out: std::collections::HashMap<String, serde_json::Value> =
        v.output.into_iter().collect();
    assert_eq!(v_out["hasNewVersion"], serde_json::json!(true));
    assert_eq!(v_out["nextVersion"], "1.3.0");
    assert_eq!(v_out["nextFullVersion"], "1.3.0+sha-def5678");

    // generate-changelog: falls back to shared commits -> Features + Bug Fixes
    let g = p
        .execute(
            "generate-changelog",
            ctx(dir.path(), "changelog"),
            &serde_json::json!({}),
        )
        .await
        .unwrap();
    assert!(g.successful, "generate-changelog failed: {:?}", g.error_message);
    let g_out: std::collections::HashMap<String, serde_json::Value> =
        g.output.into_iter().collect();
    let cats = g_out["categories"].as_array().unwrap();
    let names: Vec<&str> = cats.iter().map(|c| c["name"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["Features", "Bug Fixes"]);
}

#[tokio::test]
async fn calculate_version_empty_fails_with_exact_message() {
    let dir = tempfile::tempdir().unwrap();
    let mut p = instance(dir.path()).await;
    let v = p
        .execute(
            "calculate-version",
            ctx(dir.path(), "version"),
            &serde_json::json!({ "baseVersion": "1.2.3", "commits": [] }),
        )
        .await
        .unwrap();
    assert!(!v.successful);
    assert_eq!(v.error_message.as_deref(), Some("No commits provided for version calculation."));
}
