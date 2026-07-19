use std::sync::Arc;

use moonlit_engine::config::model::{FilesystemAccess, Permissions};
use moonlit_engine::host::{
    HostEventSink, InstanceConfig, LogLevel, PluginInstance, ReleaseContext,
};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

const FIXTURE: &[u8] = include_bytes!("fixtures/test_plugin.wasm");

struct NullSink;
impl HostEventSink for NullSink {
    fn log(&self, _s: &str, _l: LogLevel, _m: &str) {}
    fn progress(&self, _s: &str, _m: &str) {}
}

fn cfg_with_network(network: Vec<String>) -> InstanceConfig {
    InstanceConfig {
        working_directory: std::env::temp_dir(),
        permissions: Permissions {
            network,
            exec: vec!["*".to_string()],
            env: vec!["*".to_string()],
            filesystem: FilesystemAccess::ReadWrite,
        },
        config_view: serde_json::json!({}),
        env_snapshot: vec![],
    }
}

fn ctx() -> ReleaseContext {
    ReleaseContext {
        working_directory: "/work".to_string(),
        step_name: "net".to_string(),
    }
}

// build a config pointing http-get at the mock server (host-only allowlisting -> use 127.0.0.1)
fn http_cfg(server: &MockServer) -> serde_json::Value {
    let uri = server.uri(); // e.g. http://127.0.0.1:PORT
    let without_scheme = uri.strip_prefix("http://").unwrap();
    serde_json::json!({ "scheme": "http", "authority": without_scheme, "path": "/health" })
}

#[tokio::test(flavor = "multi_thread")]
async fn allowed_host_round_trips() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let eng = moonlit_engine::host::test_engine();
    let mut p = PluginInstance::instantiate(
        &eng,
        FIXTURE,
        cfg_with_network(vec!["127.0.0.1".to_string()]),
        Arc::new(NullSink),
    )
    .await
    .unwrap();
    let r = p
        .execute("http-get", ctx(), &http_cfg(&server))
        .await
        .unwrap();
    assert!(r.successful, "allowed host must round-trip: {r:?}");
    assert_eq!(
        r.output.iter().find(|(k, _)| k == "status").unwrap().1,
        serde_json::json!(200)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn denied_host_is_blocked_before_the_socket() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let eng = moonlit_engine::host::test_engine();
    // allowlist a different host -> the request to 127.0.0.1 is denied by the filter.
    let mut p = PluginInstance::instantiate(
        &eng,
        FIXTURE,
        cfg_with_network(vec!["api.github.com".to_string()]),
        Arc::new(NullSink),
    )
    .await
    .unwrap();
    let r = p
        .execute("http-get", ctx(), &http_cfg(&server))
        .await
        .unwrap();
    assert!(!r.successful, "denied host must fail the middleware");
    let received = server.received_requests().await.unwrap();
    assert!(
        received.is_empty(),
        "denied request must never reach the server (filter blocks before the socket), but server saw {} request(s)",
        received.len()
    );
}
