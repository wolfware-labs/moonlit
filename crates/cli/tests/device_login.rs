use assert_cmd::Command;
use predicates::str::contains;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn authorize_body(expires_in: u64) -> serde_json::Value {
  serde_json::json!({
    "device_code": "dev-1",
    "user_code": "ABCD-1234",
    "verification_uri": "http://example.invalid/device",
    "verification_uri_complete": "about:blank",
    "expires_in": expires_in,
    "interval": 1,
  })
}

async fn registry(expires_in: u64) -> MockServer {
  let server = MockServer::start().await;
  Mock::given(method("POST"))
    .and(path("/api/v1/device/authorize"))
    .respond_with(ResponseTemplate::new(200).set_body_json(authorize_body(expires_in)))
    .mount(&server)
    .await;
  server
}

async fn token_responds(server: &MockServer, status: u16, body: serde_json::Value, times: Option<u64>) {
  let mock = Mock::given(method("POST"))
    .and(path("/api/v1/device/token"))
    .respond_with(ResponseTemplate::new(status).set_body_json(body));
  match times {
    Some(n) => mock.up_to_n_times(n).with_priority(1).mount(server).await,
    None => mock.with_priority(2).mount(server).await,
  }
}

fn host(server: &MockServer) -> String {
  server.uri().trim_start_matches("http://").to_string()
}

fn login(server: &MockServer, home: &std::path::Path) -> assert_cmd::assert::Assert {
  Command::cargo_bin("moonlit")
    .unwrap()
    .env("MOONLIT_HOME", home)
    .args(["login", &host(server)])
    .assert()
}

fn stored_token(home: &std::path::Path, host: &str) -> Option<String> {
  let text = std::fs::read_to_string(home.join(".config/moonlit/credentials.toml")).ok()?;
  let doc: toml::Table = text.parse().ok()?;
  doc["registries"][host]["token"].as_str().map(str::to_string)
}

#[tokio::test(flavor = "multi_thread")]
async fn approved_device_flow_stores_a_bearer_token() {
  let server = registry(60).await;
  token_responds(&server, 400, serde_json::json!({ "error": "authorization_pending" }), Some(1)).await;
  token_responds(&server, 200, serde_json::json!({ "access_token": "tok-1" }), None).await;
  let home = tempfile::tempdir().unwrap();

  login(&server, home.path())
    .success()
    .stdout(contains("ABCD-1234"))
    .stdout(contains("Could not open a browser"))
    .stdout(contains("Logged in to"));
  assert_eq!(stored_token(home.path(), &host(&server)), Some("tok-1".to_string()));
}

#[tokio::test(flavor = "multi_thread")]
async fn denied_authorization_fails() {
  let server = registry(60).await;
  token_responds(&server, 400, serde_json::json!({ "error": "access_denied" }), None).await;
  let home = tempfile::tempdir().unwrap();

  login(&server, home.path()).code(1).stderr(contains("authorization denied"));
  assert_eq!(stored_token(home.path(), &host(&server)), None);
}

#[tokio::test(flavor = "multi_thread")]
async fn expired_device_code_times_out() {
  let server = registry(1).await;
  let home = tempfile::tempdir().unwrap();

  login(&server, home.path()).code(1).stderr(contains("timed out"));
}

#[tokio::test(flavor = "multi_thread")]
async fn repeated_poll_errors_give_up() {
  let server = registry(60).await;
  Mock::given(method("POST"))
    .and(path("/api/v1/device/token"))
    .respond_with(ResponseTemplate::new(500).set_body_string("down"))
    .mount(&server)
    .await;
  let home = tempfile::tempdir().unwrap();

  login(&server, home.path()).code(1).stderr(contains("error:"));
}

#[tokio::test(flavor = "multi_thread")]
async fn malformed_authorize_response_is_reported() {
  let server = MockServer::start().await;
  Mock::given(method("POST"))
    .and(path("/api/v1/device/authorize"))
    .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
    .mount(&server)
    .await;
  let home = tempfile::tempdir().unwrap();

  login(&server, home.path()).code(1).stderr(contains("bad response"));
}

#[tokio::test(flavor = "multi_thread")]
async fn rejected_authorize_request_is_unreachable() {
  let server = MockServer::start().await;
  Mock::given(method("POST"))
    .respond_with(ResponseTemplate::new(503))
    .mount(&server)
    .await;
  let home = tempfile::tempdir().unwrap();

  login(&server, home.path()).code(1).stderr(contains("could not reach"));
}

#[tokio::test(flavor = "multi_thread")]
async fn approved_token_that_cannot_be_saved_fails() {
  let server = registry(60).await;
  token_responds(&server, 200, serde_json::json!({ "access_token": "tok-1" }), None).await;
  let dir = tempfile::tempdir().unwrap();
  let home_is_a_file = dir.path().join("home");
  std::fs::write(&home_is_a_file, "").unwrap();

  login(&server, &home_is_a_file)
    .code(1)
    .stderr(contains("failed to write credentials"));
}
