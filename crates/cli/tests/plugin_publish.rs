use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;

#[test]
fn missing_file_errors_with_hint() {
  Command::cargo_bin("moonlit")
    .unwrap()
    .args([
      "plugin",
      "publish",
      "oci://reg.example.com/w/x:1",
      "--file",
      "/nonexistent/plugin.wasm",
    ])
    .assert()
    .code(2)
    .stderr(contains("no built component").and(contains("moonlit plugin build")));
}

#[test]
fn core_module_is_rejected() {
  // Smallest valid core module (magic + version 1): not a component.
  let tmp = tempfile::tempdir().unwrap();
  let path = tmp.path().join("core.wasm");
  std::fs::write(&path, [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]).unwrap();
  Command::cargo_bin("moonlit")
    .unwrap()
    .args(["plugin", "publish", "oci://reg.example.com/w/x:1", "--file"])
    .arg(&path)
    .assert()
    .code(2)
    .stderr(contains("core wasm module"));
}

#[tokio::test(flavor = "multi_thread")]
async fn component_for_another_contract_is_rejected_before_pushing() {
  let server = wiremock::MockServer::start().await;
  let fixture = concat!(env!("CARGO_MANIFEST_DIR"), "/../engine/tests/fixtures/test_plugin_v0_3.wasm");
  let reference = format!("oci://{}/acme/old:1.0.0", server.address());
  let home = tempfile::tempdir().unwrap();
  let assert = tokio::task::spawn_blocking(move || {
    Command::cargo_bin("moonlit")
      .unwrap()
      .env("MOONLIT_HOME", home.path())
      .args(["plugin", "publish", &reference, "--file", fixture])
      .assert()
  })
  .await
  .unwrap();

  assert.code(2).stderr(contains(
    "built for moonlit:plugin@0.3.0, this CLI supports moonlit:plugin@0.4.0",
  ));
  assert!(server.received_requests().await.unwrap().is_empty());
}
