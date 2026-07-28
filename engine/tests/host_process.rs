use std::sync::{Arc, Mutex};

use moonlit_engine::config::model::{FilesystemAccess, Permissions};
use moonlit_engine::host::{
    HostEventSink, InstanceConfig, LogLevel, PluginInstance, ReleaseContext,
};

const FIXTURE: &[u8] = include_bytes!("fixtures/test_plugin.wasm");

struct NullSink;
impl HostEventSink for NullSink {
    fn log(&self, _s: &str, _l: LogLevel, _m: &str) {}
    fn progress(&self, _s: &str, _m: &str) {}
}

/// Records `(step, level, message)` triples so denial-diagnostic tests can
/// assert on what the host logged.
#[derive(Default)]
struct CapturingSink {
    events: Mutex<Vec<(String, LogLevel, String)>>,
}

impl HostEventSink for CapturingSink {
    fn log(&self, step: &str, level: LogLevel, message: &str) {
        self.events
            .lock()
            .unwrap()
            .push((step.to_string(), level, message.to_string()));
    }
    fn progress(&self, _s: &str, _m: &str) {}
}

fn cfg_with_exec(exec: Vec<String>) -> InstanceConfig {
    InstanceConfig {
        working_directory: std::env::temp_dir(),
        permissions: Permissions {
            network: vec!["*".to_string()],
            exec,
            env: vec!["*".to_string()],
            filesystem: FilesystemAccess::ReadWrite,
        },
        config_view: serde_json::json!({}),
        env_snapshot: vec![],
    }
}

fn ctx(s: &str) -> ReleaseContext {
    ReleaseContext {
        working_directory: "/work".to_string(),
        step_name: s.to_string(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn run_process_succeeds_when_program_permitted() {
    let eng = moonlit_engine::host::test_engine();
    let mut p = PluginInstance::instantiate(
        &eng,
        FIXTURE,
        cfg_with_exec(vec!["echo".to_string()]),
        Arc::new(NullSink),
    )
    .await
    .unwrap();
    let r = p
        .execute("run-process", ctx("run"), &serde_json::json!({}))
        .await
        .unwrap();
    assert!(r.successful, "run-process should succeed: {r:?}");
    assert_eq!(
        r.output.iter().find(|(k, _)| k == "exit_code").unwrap().1,
        serde_json::json!(0)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn run_process_denied_when_program_not_permitted() {
    let eng = moonlit_engine::host::test_engine();
    // allowlist permits only "ls"; the guest runs "echo" -> denied.
    let sink = Arc::new(CapturingSink::default());
    let mut p = PluginInstance::instantiate(
        &eng,
        FIXTURE,
        cfg_with_exec(vec!["ls".to_string()]),
        sink.clone(),
    )
    .await
    .unwrap();
    let r = p
        .execute("run-process", ctx("run"), &serde_json::json!({}))
        .await
        .unwrap();
    assert!(!r.successful, "denied exec must fail the middleware");
    assert!(r.error_message.unwrap().contains("not permitted"));

    let events = sink.events.lock().unwrap();
    assert!(
        events.iter().any(|(_, level, msg)| *level == LogLevel::Warn
            && msg.contains("echo")
            && msg.contains("permissions.exec")),
        "expected a Warn event naming the denied program and permissions.exec, got: {events:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn spawn_stream_delivers_lines_live_in_order() {
    let eng = moonlit_engine::host::test_engine();
    let mut p = PluginInstance::instantiate(
        &eng,
        FIXTURE,
        cfg_with_exec(vec!["sh".to_string()]),
        Arc::new(NullSink),
    )
    .await
    .unwrap();
    let r = p
        .execute("spawn-stream", ctx("stream"), &serde_json::json!({}))
        .await
        .unwrap();
    assert!(r.successful, "spawn-stream should succeed: {r:?}");
    assert_eq!(
        r.output.iter().find(|(k, _)| k == "lines").unwrap().1,
        serde_json::json!("a,b,c")
    );
    assert_eq!(
        r.output.iter().find(|(k, _)| k == "exit_code").unwrap().1,
        serde_json::json!(0)
    );
}
