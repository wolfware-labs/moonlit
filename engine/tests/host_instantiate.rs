use std::sync::Arc;

use moonlit_engine::config::model::Permissions;
use moonlit_engine::host::{HostEventSink, InstanceConfig, LogLevel, PluginInstance};

const FIXTURE: &[u8] = include_bytes!("fixtures/test_plugin.wasm");

struct NullSink;
impl HostEventSink for NullSink {
    fn log(&self, _step: &str, _level: LogLevel, _message: &str) {}
    fn progress(&self, _step: &str, _message: &str) {}
}

fn full_trust_config() -> InstanceConfig {
    InstanceConfig {
        working_directory: std::env::temp_dir(),
        permissions: Permissions::full_trust(),
        config_view: serde_json::json!({}),
        env_snapshot: vec![],
    }
}

async fn engine() -> wasmtime::Engine {
    // reuse the crate's builder via a fresh instance path
    moonlit_engine::host::test_engine()
}

async fn instance() -> PluginInstance {
    let eng = engine().await;
    PluginInstance::instantiate(&eng, FIXTURE, full_trust_config(), Arc::new(NullSink))
        .await
        .expect("instantiation (wasi:http + custom imports linked) must succeed")
}

#[tokio::test]
async fn init_ok_and_failinit_err() {
    let mut p = instance().await;
    let meta = p.init(&serde_json::json!({})).await.expect("init Ok");
    assert_eq!(meta.name, "test-plugin");

    let mut p2 = instance().await;
    let err = p2.init(&serde_json::json!({ "failInit": true })).await;
    assert!(err.is_err(), "failInit must produce Err");
}

#[tokio::test]
async fn lists_all_middlewares() {
    let mut p = instance().await;
    let mws = p.list_middlewares().await.unwrap();
    let names: Vec<_> = mws.iter().map(|m| m.name.as_str()).collect();
    for expected in [
        "log-and-output",
        "run-process",
        "spawn-stream",
        "http-get",
        "boom",
    ] {
        assert!(names.contains(&expected), "missing middleware {expected}");
    }
}
