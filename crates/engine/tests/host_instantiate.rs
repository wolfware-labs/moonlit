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
async fn describe_and_list_carry_icon_and_io_schema() {
    let mut p = instance().await;

    // ABI 0.3.0: describe() carries the plugin icon as a data URI.
    let meta = p.describe().await.expect("describe Ok");
    let icon = meta.icon.expect("test fixture must carry an icon");
    assert!(
        icon.starts_with("data:image/png;base64,"),
        "icon must be a PNG data URI; got {icon:.32}"
    );

    // Each middleware carries optional input/output schemas: log-and-output
    // declares both, run-process leaves them absent — both paths must survive
    // the host mapping.
    let mws = p.list_middlewares().await.unwrap();
    let log = mws.iter().find(|m| m.name == "log-and-output").unwrap();
    assert!(
        log.input_schema
            .as_deref()
            .is_some_and(|s| s.contains("2020-12")),
        "log-and-output must carry a draft-2020-12 input schema"
    );
    assert!(
        log.output_schema
            .as_deref()
            .is_some_and(|s| s.contains("2020-12")),
        "log-and-output must carry a draft-2020-12 output schema"
    );
    let run = mws.iter().find(|m| m.name == "run-process").unwrap();
    assert!(
        run.input_schema.is_none(),
        "run-process must have no input schema"
    );
    assert!(
        run.output_schema.is_none(),
        "run-process must have no output schema"
    );
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
