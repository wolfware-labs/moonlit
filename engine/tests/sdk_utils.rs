//! Proves the moonlit-plugin-sdk utility modules (process/http/env) run for real
//! inside the engine host: a real subprocess, an HTTP round-trip against a local
//! mock server (incl. gzip inflate), and a permission-filtered env read.

use std::io::Write;
use std::sync::Arc;

use moonlit_engine::config::model::{FilesystemAccess, Permissions};
use moonlit_engine::host::{
    HostEventSink, InstanceConfig, LogLevel, PluginInstance, ReleaseContext,
};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const FIXTURE: &[u8] = include_bytes!("fixtures/sdk_sample.wasm");

struct NullSink;
impl HostEventSink for NullSink {
    fn log(&self, _s: &str, _l: LogLevel, _m: &str) {}
    fn progress(&self, _s: &str, _m: &str) {}
}

fn perms(network: Vec<&str>, exec: Vec<&str>) -> Permissions {
    Permissions {
        network: network.into_iter().map(String::from).collect(),
        exec: exec.into_iter().map(String::from).collect(),
        env: vec!["*".to_string()],
        filesystem: FilesystemAccess::ReadWrite,
    }
}

fn cfg(permissions: Permissions, env_snapshot: Vec<(String, String)>) -> InstanceConfig {
    InstanceConfig {
        working_directory: std::env::temp_dir(),
        permissions,
        config_view: serde_json::json!({}),
        env_snapshot,
    }
}

fn ctx(step: &str) -> ReleaseContext {
    ReleaseContext {
        working_directory: "/work".to_string(),
        step_name: step.to_string(),
    }
}

async fn instance(config: InstanceConfig) -> PluginInstance {
    let eng = moonlit_engine::host::test_engine();
    PluginInstance::instantiate(&eng, FIXTURE, config, Arc::new(NullSink))
        .await
        .expect("sdk-sample instantiates")
}

#[tokio::test(flavor = "multi_thread")]
async fn run_echo_executes_a_real_subprocess() {
    let mut p = instance(cfg(perms(vec![], vec!["echo"]), vec![])).await;
    let r = p
        .execute("run-echo", ctx("run"), &serde_json::json!({}))
        .await
        .expect("execute Ok");
    assert!(r.successful, "{r:?}");
    let map: std::collections::HashMap<_, _> = r.output.into_iter().collect();
    assert_eq!(map["exit_code"], serde_json::json!(0));
    assert_eq!(map["stdout"], serde_json::json!("hello"));
}

#[tokio::test(flavor = "multi_thread")]
async fn spawn_echo_streams_lines() {
    let mut p = instance(cfg(perms(vec![], vec!["echo"]), vec![])).await;
    let r = p
        .execute("spawn-echo", ctx("spawn"), &serde_json::json!({}))
        .await
        .expect("execute Ok");
    assert!(r.successful, "{r:?}");
    let map: std::collections::HashMap<_, _> = r.output.into_iter().collect();
    assert_eq!(map["exit_code"], serde_json::json!(0));
    assert_eq!(map["lines"], serde_json::json!("hello"));
}

#[tokio::test(flavor = "multi_thread")]
async fn http_get_round_trips_against_mock_server() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/data"))
        .respond_with(ResponseTemplate::new(200).set_body_string("pong"))
        .mount(&server)
        .await;
    let authority = server.uri().strip_prefix("http://").unwrap().to_string();
    let step_cfg = serde_json::json!({ "scheme": "http", "authority": authority, "path": "/data" });

    let mut p = instance(cfg(perms(vec!["127.0.0.1"], vec![]), vec![])).await;
    let r = p
        .execute("http-get", ctx("http"), &step_cfg)
        .await
        .expect("execute Ok");
    assert!(r.successful, "{r:?}");
    let map: std::collections::HashMap<_, _> = r.output.into_iter().collect();
    assert_eq!(map["status"], serde_json::json!(200));
    assert_eq!(map["body"], serde_json::json!("pong"));
}

#[tokio::test(flavor = "multi_thread")]
async fn http_get_inflates_gzip_response() {
    let server = MockServer::start().await;
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    enc.write_all(b"zipped").unwrap();
    let gz = enc.finish().unwrap();
    Mock::given(method("GET"))
        .and(path("/gz"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-encoding", "gzip")
                .set_body_bytes(gz),
        )
        .mount(&server)
        .await;
    let authority = server.uri().strip_prefix("http://").unwrap().to_string();
    let step_cfg = serde_json::json!({ "scheme": "http", "authority": authority, "path": "/gz" });

    let mut p = instance(cfg(perms(vec!["127.0.0.1"], vec![]), vec![])).await;
    let r = p
        .execute("http-get", ctx("gz"), &step_cfg)
        .await
        .expect("execute Ok");
    assert!(r.successful, "{r:?}");
    let map: std::collections::HashMap<_, _> = r.output.into_iter().collect();
    assert_eq!(map["body"], serde_json::json!("zipped"));
}

#[tokio::test(flavor = "multi_thread")]
async fn read_env_sees_permission_filtered_env() {
    let env = vec![("SAMPLE_ENV".to_string(), "xyz".to_string())];
    let mut p = instance(cfg(perms(vec![], vec![]), env)).await;
    let r = p
        .execute("read-env", ctx("env"), &serde_json::json!({}))
        .await
        .expect("execute Ok");
    assert!(r.successful, "{r:?}");
    let map: std::collections::HashMap<_, _> = r.output.into_iter().collect();
    assert_eq!(map["SAMPLE_ENV"], serde_json::json!("xyz"));
}
