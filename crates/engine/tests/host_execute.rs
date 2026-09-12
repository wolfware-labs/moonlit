use std::sync::{Arc, Mutex};

use moonlit_engine::config::model::Permissions;
use moonlit_engine::host::{
    HostError, HostEventSink, InstanceConfig, LogLevel, PluginInstance, ReleaseContext,
};

const FIXTURE: &[u8] = include_bytes!("fixtures/test_plugin.wasm");

#[derive(Default)]
struct RecordingSink {
    logs: Mutex<Vec<String>>,
    progress: Mutex<Vec<String>>,
}
impl HostEventSink for RecordingSink {
    fn log(&self, step: &str, level: LogLevel, message: &str) {
        self.logs
            .lock()
            .unwrap()
            .push(format!("{step}:{level:?}:{message}"));
    }
    fn progress(&self, step: &str, message: &str) {
        self.progress
            .lock()
            .unwrap()
            .push(format!("{step}:{message}"));
    }
}

fn cfg() -> InstanceConfig {
    InstanceConfig {
        working_directory: std::env::temp_dir(),
        permissions: Permissions::full_trust(),
        config_view: serde_json::json!({ "plugin": { "name": "test-plugin" } }),
        env_snapshot: vec![],
    }
}

fn ctx(step: &str) -> ReleaseContext {
    ReleaseContext {
        working_directory: "/work".to_string(),
        step_name: step.to_string(),
    }
}

#[tokio::test]
async fn execute_log_and_output_returns_result_and_emits_events() {
    let eng = moonlit_engine::host::test_engine();
    let sink = Arc::new(RecordingSink::default());
    let mut p = PluginInstance::instantiate(&eng, FIXTURE, cfg(), sink.clone())
        .await
        .unwrap();

    let result = p
        .execute(
            "log-and-output",
            ctx("compile"),
            &serde_json::json!({ "k": 1 }),
        )
        .await
        .expect("execute Ok");

    assert!(result.successful);
    assert_eq!(result.warnings, vec!["a benign warning".to_string()]);
    // echoed_config parsed back into a structured value
    let echoed = &result
        .output
        .iter()
        .find(|(k, _)| k == "echoed_config")
        .unwrap()
        .1;
    assert_eq!(echoed["k"], serde_json::json!(1));
    // get-config served from the injected config view
    let cfg_seen = &result
        .output
        .iter()
        .find(|(k, _)| k == "cfg_seen")
        .unwrap()
        .1;
    assert_eq!(cfg_seen, &serde_json::json!("test-plugin"));
    // events routed to the sink, tagged with the current step
    assert!(
        sink.logs
            .lock()
            .unwrap()
            .iter()
            .any(|l| l.starts_with("compile:Info:"))
    );
    assert!(
        sink.progress
            .lock()
            .unwrap()
            .iter()
            .any(|p| p == "compile:halfway there")
    );
}

#[tokio::test]
async fn panicking_middleware_surfaces_as_trap() {
    let eng = moonlit_engine::host::test_engine();
    let mut p =
        PluginInstance::instantiate(&eng, FIXTURE, cfg(), Arc::new(RecordingSink::default()))
            .await
            .unwrap();
    let err = p
        .execute("boom", ctx("boom"), &serde_json::json!({}))
        .await
        .unwrap_err();
    assert!(
        matches!(err, HostError::Trap { .. }),
        "guest panic must be HostError::Trap"
    );
}
