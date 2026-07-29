//! Proves the moonlit-sdk sample compiles to a real component the engine
//! host runs: init metadata, middleware discovery, a coerced execute with
//! outputs + get-config, and a failure path.

use std::sync::Arc;

use moonlit_engine::config::model::Permissions;
use moonlit_engine::host::{
    HostEventSink, InstanceConfig, LogLevel, PluginInstance, ReleaseContext,
};

const FIXTURE: &[u8] = include_bytes!("fixtures/sdk_sample.wasm");

struct NullSink;
impl HostEventSink for NullSink {
    fn log(&self, _step: &str, _level: LogLevel, _message: &str) {}
    fn progress(&self, _step: &str, _message: &str) {}
}

fn cfg() -> InstanceConfig {
    InstanceConfig {
        working_directory: std::env::temp_dir(),
        permissions: Permissions::full_trust(),
        config_view: serde_json::json!({ "plugin": { "name": "sdk-sample" } }),
        env_snapshot: vec![],
    }
}

fn ctx(step: &str) -> ReleaseContext {
    ReleaseContext {
        working_directory: "/work".to_string(),
        step_name: step.to_string(),
    }
}

async fn instance() -> PluginInstance {
    let eng = moonlit_engine::host::test_engine();
    PluginInstance::instantiate(&eng, FIXTURE, cfg(), Arc::new(NullSink))
        .await
        .expect("sdk-sample instantiates")
}

#[tokio::test]
async fn init_reports_sdk_metadata() {
    let mut p = instance().await;
    let meta = p.init(&serde_json::json!({})).await.expect("init Ok");
    assert_eq!(meta.name, "sdk-sample");
    assert_eq!(meta.version, "0.1.0");
}

#[tokio::test]
async fn lists_sdk_middlewares() {
    let mut p = instance().await;
    let mws = p.list_middlewares().await.unwrap();
    let names: Vec<_> = mws.iter().map(|m| m.name.as_str()).collect();
    assert!(names.contains(&"echo"));
    assert!(names.contains(&"fail"));
    let echo = mws.iter().find(|m| m.name == "echo").unwrap();
    assert_eq!(echo.description, "echoes config and reads plugin:name");
}

#[tokio::test]
async fn execute_echo_coerces_config_and_reads_get_config() {
    let mut p = instance().await;
    // `times` sent as a string, as the engine always does — the SDK coerces it.
    let result = p
        .execute(
            "echo",
            ctx("compile"),
            &serde_json::json!({ "times": "3", "label": "hi" }),
        )
        .await
        .expect("execute Ok");
    assert!(result.successful);
    assert_eq!(result.warnings, vec!["sample warning".to_string()]);
    let map: std::collections::HashMap<_, _> = result.output.into_iter().collect();
    assert_eq!(map["times"], serde_json::json!(3));
    assert_eq!(map["label"], serde_json::json!("hi"));
    assert_eq!(map["step"], serde_json::json!("compile"));
    assert_eq!(map["plugin_name"], serde_json::json!("sdk-sample"));
}

#[tokio::test]
async fn execute_fail_reports_error() {
    let mut p = instance().await;
    let result = p
        .execute("fail", ctx("s"), &serde_json::json!({}))
        .await
        .expect("execute call Ok (result carries the failure)");
    assert!(!result.successful);
    assert_eq!(result.error_message.as_deref(), Some("intentional failure"));
}
