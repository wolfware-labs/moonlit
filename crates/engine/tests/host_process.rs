use std::sync::{Arc, Mutex};

use moonlit_engine::logging::LogLevel;
use moonlit_engine::pipeline::config::{FilesystemAccess, Permissions};
use moonlit_engine::plugin::host::{HostEventSink, ReleaseContext};
use moonlit_engine::plugin::{Plugin, PluginInstanceConfig};

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
    self
      .events
      .lock()
      .unwrap()
      .push((step.to_string(), level, message.to_string()));
  }
  fn progress(&self, _s: &str, _m: &str) {}
}

fn cfg_with_exec(exec: Vec<String>) -> PluginInstanceConfig {
  PluginInstanceConfig {
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
  let eng = moonlit_engine::engine::Engine::try_default().unwrap();
  let mut p = Plugin::instantiate(&eng, FIXTURE, cfg_with_exec(vec!["echo".to_string()]), Arc::new(NullSink))
    .await
    .unwrap();
  let r = p.execute("run-process", ctx("run"), &serde_json::json!({})).await.unwrap();
  assert!(r.successful, "run-process should succeed: {r:?}");
  assert_eq!(
    r.output.iter().find(|(k, _)| k == "exit_code").unwrap().1,
    serde_json::json!(0)
  );
}

#[tokio::test(flavor = "multi_thread")]
async fn run_process_denied_when_program_not_permitted() {
  let eng = moonlit_engine::engine::Engine::try_default().unwrap();
  // allowlist permits only "ls"; the guest runs "echo" -> denied.
  let sink = Arc::new(CapturingSink::default());
  let mut p = Plugin::instantiate(&eng, FIXTURE, cfg_with_exec(vec!["ls".to_string()]), sink.clone())
    .await
    .unwrap();
  let r = p.execute("run-process", ctx("run"), &serde_json::json!({})).await.unwrap();
  assert!(!r.successful, "denied exec must fail the middleware");
  assert!(r.error_message.unwrap().contains("permissions.exec"));

  let events = sink.events.lock().unwrap();
  assert!(
    events
      .iter()
      .any(|(_, level, msg)| *level == LogLevel::Warn && msg.contains("echo") && msg.contains("permissions.exec")),
    "expected a Warn event naming the denied program and permissions.exec, got: {events:?}"
  );
}

#[tokio::test(flavor = "multi_thread")]
async fn spawn_stream_delivers_lines_live_in_order() {
  let eng = moonlit_engine::engine::Engine::try_default().unwrap();
  let mut p = Plugin::instantiate(&eng, FIXTURE, cfg_with_exec(vec!["sh".to_string()]), Arc::new(NullSink))
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

#[tokio::test(flavor = "multi_thread")]
async fn denied_exec_fails_step_even_if_plugin_succeeds() {
  let eng = moonlit_engine::engine::Engine::try_default().unwrap();
  let mut p = Plugin::instantiate(&eng, FIXTURE, cfg_with_exec(vec!["ls".to_string()]), Arc::new(NullSink))
    .await
    .unwrap();
  let meta = p.describe().await.unwrap();
  p.set_declared(meta.permissions);
  let r = p
    .execute("try-exec", ctx("tag"), &serde_json::json!({ "program": "git" }))
    .await
    .unwrap();
  assert!(!r.successful, "a blocked program must fail the step: {r:?}");
  let msg = r.error_message.unwrap();
  assert!(msg.contains("blocked from running 'git'"), "{msg}");
  assert!(msg.contains("permissions.exec"), "{msg}");
  assert!(msg.contains("Only for the tag middleware."), "{msg}");
}

#[tokio::test(flavor = "multi_thread")]
async fn granted_exec_does_not_fail() {
  let eng = moonlit_engine::engine::Engine::try_default().unwrap();
  let mut p = Plugin::instantiate(&eng, FIXTURE, cfg_with_exec(vec!["git".to_string()]), Arc::new(NullSink))
    .await
    .unwrap();
  let meta = p.describe().await.unwrap();
  p.set_declared(meta.permissions);
  let r = p
    .execute("try-exec", ctx("tag"), &serde_json::json!({ "program": "git" }))
    .await
    .unwrap();
  assert!(r.successful, "granted exec must not fail: {r:?}");
  assert!(r.error_message.is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn denials_do_not_leak_into_the_next_execute() {
  let eng = moonlit_engine::engine::Engine::try_default().unwrap();
  let mut p = Plugin::instantiate(&eng, FIXTURE, cfg_with_exec(vec!["ls".to_string()]), Arc::new(NullSink))
    .await
    .unwrap();
  let first = p
    .execute("try-exec", ctx("a"), &serde_json::json!({ "program": "git" }))
    .await
    .unwrap();
  let second = p
    .execute("try-exec", ctx("b"), &serde_json::json!({ "program": "ls" }))
    .await
    .unwrap();
  assert!(!first.successful);
  assert!(second.successful, "{second:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_denial_during_init_fails_init() {
  let eng = moonlit_engine::engine::Engine::try_default().unwrap();
  let mut p = Plugin::instantiate(&eng, FIXTURE, cfg_with_exec(vec!["ls".to_string()]), Arc::new(NullSink))
    .await
    .unwrap();
  let err = p
    .init(&serde_json::json!({ "execOnInit": "git" }))
    .await
    .expect_err("a blocked program must fail init");
  assert!(err.contains("blocked from running 'git'"), "{err}");
  assert!(err.contains("permissions.exec"), "{err}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_denial_while_listing_middlewares_fails_the_listing() {
  let eng = moonlit_engine::engine::Engine::try_default().unwrap();
  let mut p = Plugin::instantiate(&eng, FIXTURE, cfg_with_exec(vec!["ls".to_string()]), Arc::new(NullSink))
    .await
    .unwrap();
  p.init(&serde_json::json!({ "execOnList": "git" })).await.unwrap();
  let err = p
    .list_middlewares()
    .await
    .expect_err("a blocked program must fail the listing");
  let msg = err.to_string();
  assert!(msg.contains("blocked from running 'git'"), "{msg}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_granted_program_during_init_and_listing_does_not_fail() {
  let eng = moonlit_engine::engine::Engine::try_default().unwrap();
  let mut p = Plugin::instantiate(&eng, FIXTURE, cfg_with_exec(vec!["git".to_string()]), Arc::new(NullSink))
    .await
    .unwrap();
  p.init(&serde_json::json!({ "execOnInit": "git", "execOnList": "git" }))
    .await
    .unwrap();
  assert!(!p.list_middlewares().await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_denial_during_a_failed_init_keeps_the_plugins_own_error() {
  let eng = moonlit_engine::engine::Engine::try_default().unwrap();
  let mut p = Plugin::instantiate(&eng, FIXTURE, cfg_with_exec(vec!["ls".to_string()]), Arc::new(NullSink))
    .await
    .unwrap();
  let err = p
    .init(&serde_json::json!({ "execOnInit": "git", "failInit": true }))
    .await
    .expect_err("init must fail");
  assert!(
    err.starts_with("init failed on request (failInit=true); blocked from running 'git'"),
    "{err}"
  );
}
