#[path = "support/oci_mock.rs"]
mod oci_mock;

use assert_cmd::Command;
use predicates::str::contains;
use serde_json::{Value, json};
use std::path::Path;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const OLD_WORLD: &str = "moonlit:plugin@0.3.0";
const WORLD: &str = "moonlit:plugin@0.4.0";

fn fixture(name: &str) -> Vec<u8> {
  std::fs::read(
    Path::new(env!("CARGO_MANIFEST_DIR"))
      .join("../engine/tests/fixtures")
      .join(name),
  )
  .expect("fixture exists")
}

async fn mount(server: &MockServer, markers: &[&str], data: Value) {
  let mut mock = Mock::given(method("POST")).and(path("/graphql"));
  for marker in markers {
    mock = mock.and(body_string_contains(*marker));
  }
  mock
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": data })))
    .mount(server)
    .await;
}

async fn registry_with_upgrade() -> MockServer {
  registry_with_versions(&["2.0.0"], &[]).await
}

async fn registry_with_versions(compatible: &[&str], broken: &[&str]) -> MockServer {
  let server = MockServer::start().await;
  let listed: Vec<Value> = std::iter::once("1.0.0")
    .chain(compatible.iter().copied())
    .chain(broken.iter().copied())
    .map(|v| json!({ "version": v, "yanked": false, "status": "VERIFIED" }))
    .collect();
  mount(&server, &["query versions("], json!({ "package": { "versions": listed } })).await;
  for version in broken {
    Mock::given(method("POST"))
      .and(path("/graphql"))
      .and(body_string_contains("query packageVersion("))
      .and(body_string_contains(format!("\"version\":\"{version}\"")))
      .respond_with(ResponseTemplate::new(500))
      .mount(&server)
      .await;
  }
  let versions = std::iter::once(("1.0.0", OLD_WORLD)).chain(compatible.iter().map(|v| (*v, WORLD)));
  for (version, world) in versions {
    mount(
      &server,
      &["query packageVersion(", &format!("\"version\":\"{version}\"")],
      json!({ "packageVersion": { "version": version, "worldVersion": world, "status": "VERIFIED", "middlewares": [] } }),
    )
    .await;
  }
  for repo in ["acme/old-a", "acme/old-b"] {
    for tag in ["1.0.0", "latest"] {
      oci_mock::serve_plugin(&server, repo, tag, &fixture("test_plugin_v0_3.wasm"), OLD_WORLD).await;
    }
  }
  server
}

fn run_stderr(dir: &Path) -> String {
  let output = moonlit_in(dir)
    .args(["run", "--output", "plain"])
    .assert()
    .code(3)
    .get_output()
    .clone();
  String::from_utf8_lossy(&output.stderr).into_owned()
}

fn oci_release(dir: &Path, server: &MockServer) {
  oci_release_tagged(dir, server, "1.0.0");
}

fn oci_release_tagged(dir: &Path, server: &MockServer, tag: &str) {
  let host = server.address();
  let yaml = format!(
    "name: demo\nplugins:\n  - name: old-a\n    url: oci://{host}/acme/old-a:{tag}\n  - name: old-b\n    url: oci://{host}/acme/old-b:{tag}\nstages:\n  build:\n    - run: old-a.log-and-output\n"
  );
  std::fs::write(dir.join("release.yml"), yaml).unwrap();
}

fn fixture_wasm_url() -> String {
  let p = Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../engine/tests/fixtures/test_plugin.wasm")
    .canonicalize()
    .expect("fixture wasm exists");
  format!("file://{}", p.display())
}

fn unwrapped_stderr(output: &std::process::Output) -> String {
  String::from_utf8_lossy(&output.stderr)
    .split_whitespace()
    .filter(|word| *word != "│")
    .collect::<Vec<_>>()
    .join(" ")
}

fn write_release(path: &Path) {
  let yaml = format!(
    "name: demo\nplugins:\n  - name: tp\n    url: {}\n    permissions:\n      network: [api.example.com]\nstages:\n  build:\n    - name: s1\n      run: tp.log-and-output\n  other:\n    - name: s2\n      run: tp.log-and-output\n",
    fixture_wasm_url()
  );
  std::fs::write(path, yaml).unwrap();
}

fn moonlit_in(dir: &Path) -> Command {
  let mut cmd = Command::cargo_bin("moonlit").unwrap();
  cmd.current_dir(dir);
  cmd
}

#[test]
fn run_finds_release_yaml_in_the_current_directory() {
  let dir = tempfile::tempdir().unwrap();
  write_release(&dir.path().join("release.yaml"));
  moonlit_in(dir.path())
    .args(["run", "--output", "plain", "-v", "-s", "build", "-a", "who=world"])
    .assert()
    .success()
    .stderr(contains("stages: build"))
    .stderr(contains("s1"));
}

#[test]
fn validate_finds_release_yml_in_the_current_directory() {
  let dir = tempfile::tempdir().unwrap();
  write_release(&dir.path().join("release.yml"));
  moonlit_in(dir.path())
    .args(["validate", "--output", "plain"])
    .assert()
    .success()
    .stderr(contains("Configuration valid"));
}

#[test]
fn pretty_run_succeeds_without_a_terminal() {
  let dir = tempfile::tempdir().unwrap();
  write_release(&dir.path().join("release.yml"));
  moonlit_in(dir.path()).args(["run", "--output", "pretty"]).assert().success();
}

#[test]
fn run_without_a_pipeline_file_names_the_defaults() {
  let dir = tempfile::tempdir().unwrap();
  let assert = moonlit_in(dir.path()).args(["run", "--output", "plain"]).assert().code(2);
  let stderr = unwrapped_stderr(assert.get_output());
  assert!(stderr.contains("No pipeline file found"), "{stderr}");
  assert!(stderr.contains("(looked for release.yml, release.yaml)"), "{stderr}");
}

#[test]
fn run_with_a_missing_file_reports_it() {
  let dir = tempfile::tempdir().unwrap();
  let assert = moonlit_in(dir.path())
    .args(["run", "--output", "plain", "-f", "missing.yml"])
    .assert()
    .code(2);
  let stderr = unwrapped_stderr(assert.get_output());
  assert!(stderr.contains("missing.yml' does not exist."), "{stderr}");
}

#[test]
fn json_mode_reports_errors_as_a_json_line() {
  let dir = tempfile::tempdir().unwrap();
  let out = moonlit_in(dir.path())
    .args(["run", "--output", "json", "-f", "missing.yml"])
    .assert()
    .code(2)
    .get_output()
    .stdout
    .clone();
  let line: serde_json::Value = serde_json::from_slice(&out).unwrap();
  assert_eq!(line["type"], "error");
  assert_eq!(line["exit_code"], 2);
}

#[test]
fn json_mode_reports_pipeline_errors_as_a_json_line() {
  let dir = tempfile::tempdir().unwrap();
  std::fs::write(dir.path().join("release.yml"), "pluigns: []\n").unwrap();
  let out = moonlit_in(dir.path())
    .args(["run", "--output", "json"])
    .assert()
    .code(2)
    .get_output()
    .stdout
    .clone();
  let text = String::from_utf8(out).unwrap();
  assert!(text.lines().any(|l| l.contains(r#""type":"error""#)), "{text}");
}

#[test]
fn invalid_arg_is_rejected_by_the_parser() {
  let dir = tempfile::tempdir().unwrap();
  moonlit_in(dir.path())
    .args(["run", "-a", "novalue"])
    .assert()
    .code(2)
    .stderr(contains("expected key=value"));
}

#[test]
fn unreadable_pipeline_file_is_reported() {
  let dir = tempfile::tempdir().unwrap();
  std::fs::write(dir.path().join("release.yml"), [0xff, 0xfe, 0xfd]).unwrap();
  moonlit_in(dir.path())
    .args(["run", "--output", "plain"])
    .assert()
    .code(2)
    .stderr(contains("Error while reading"));
}

#[tokio::test(flavor = "multi_thread")]
async fn incompatible_error_lists_each_plugin_with_suggestion() {
  let server = registry_with_upgrade().await;
  let dir = tempfile::tempdir().unwrap();
  oci_release(dir.path(), &server);
  let host = server.address().to_string();
  let output = tokio::task::spawn_blocking({
    let dir = dir.path().to_path_buf();
    move || {
      moonlit_in(&dir)
        .args(["run", "--output", "plain"])
        .assert()
        .code(3)
        .get_output()
        .clone()
    }
  })
  .await
  .unwrap();
  let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
  for name in ["old-a", "old-b"] {
    assert!(
      stderr.contains(&format!("✗ {name}  oci://{host}/acme/{name}:1.0.0")),
      "{stderr}"
    );
  }
  assert_eq!(
    stderr
      .matches("built for moonlit:plugin@0.3.0, this CLI supports moonlit:plugin@0.4.0")
      .count(),
    2,
    "{stderr}"
  );
  assert_eq!(
    stderr
      .matches("use 2.0.0 instead (run `moonlit doctor --major` to fix)")
      .count(),
    2,
    "{stderr}"
  );
}

#[tokio::test(flavor = "multi_thread")]
async fn same_major_suggestion_points_at_plain_doctor() {
  let server = registry_with_versions(&["1.1.0", "2.0.0"], &[]).await;
  let dir = tempfile::tempdir().unwrap();
  oci_release(dir.path(), &server);
  let path = dir.path().to_path_buf();
  let stderr = tokio::task::spawn_blocking(move || run_stderr(&path)).await.unwrap();
  assert_eq!(
    stderr.matches("use 1.1.0 instead (run `moonlit doctor` to fix)").count(),
    2,
    "{stderr}"
  );
  assert!(!stderr.contains("--major"), "{stderr}");
}

#[tokio::test(flavor = "multi_thread")]
async fn non_semver_tag_goes_straight_to_the_cross_major_lookup() {
  let server = registry_with_versions(&["1.1.0", "2.0.0"], &[]).await;
  let dir = tempfile::tempdir().unwrap();
  oci_release_tagged(dir.path(), &server, "latest");
  let path = dir.path().to_path_buf();
  let stderr = tokio::task::spawn_blocking(move || run_stderr(&path)).await.unwrap();
  assert_eq!(
    stderr
      .matches("use 2.0.0 instead (run `moonlit doctor --major` to fix)")
      .count(),
    2,
    "{stderr}"
  );
}

#[tokio::test(flavor = "multi_thread")]
async fn registry_error_on_the_same_major_lookup_gives_no_suggestion() {
  let server = registry_with_versions(&["2.0.0"], &["1.1.0"]).await;
  let dir = tempfile::tempdir().unwrap();
  oci_release(dir.path(), &server);
  let path = dir.path().to_path_buf();
  let stderr = tokio::task::spawn_blocking(move || run_stderr(&path)).await.unwrap();
  assert!(stderr.contains("✗ old-a"), "{stderr}");
  assert!(!stderr.contains("instead"), "{stderr}");
}

#[tokio::test(flavor = "multi_thread")]
async fn oci_host_that_is_not_a_moonlit_registry_gets_the_rebuild_hint() {
  let server = MockServer::start().await;
  Mock::given(method("POST"))
    .and(path("/graphql"))
    .respond_with(ResponseTemplate::new(404))
    .mount(&server)
    .await;
  oci_mock::serve_plugin(&server, "acme/old-a", "1.0.0", &fixture("test_plugin_v0_3.wasm"), OLD_WORLD).await;
  oci_mock::serve_plugin(&server, "acme/old-b", "1.0.0", &fixture("test_plugin_v0_3.wasm"), OLD_WORLD).await;
  let dir = tempfile::tempdir().unwrap();
  oci_release(dir.path(), &server);
  let path = dir.path().to_path_buf();
  let stderr = tokio::task::spawn_blocking(move || run_stderr(&path)).await.unwrap();
  assert_eq!(
    stderr
      .matches("rebuild it against moonlit-pdk for moonlit:plugin@0.4.0")
      .count(),
    2,
    "{stderr}"
  );
}

#[tokio::test(flavor = "multi_thread")]
async fn incompatible_error_without_registry_still_names_worlds() {
  let server = registry_with_upgrade().await;
  server.reset().await;
  Mock::given(method("POST"))
    .and(path("/graphql"))
    .respond_with(ResponseTemplate::new(500))
    .mount(&server)
    .await;
  oci_mock::serve_plugin(&server, "acme/old-a", "1.0.0", &fixture("test_plugin_v0_3.wasm"), OLD_WORLD).await;
  oci_mock::serve_plugin(&server, "acme/old-b", "1.0.0", &fixture("test_plugin_v0_3.wasm"), OLD_WORLD).await;
  let dir = tempfile::tempdir().unwrap();
  oci_release(dir.path(), &server);
  let output = tokio::task::spawn_blocking({
    let dir = dir.path().to_path_buf();
    move || {
      moonlit_in(&dir)
        .args(["run", "--output", "plain"])
        .assert()
        .code(3)
        .get_output()
        .clone()
    }
  })
  .await
  .unwrap();
  let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
  assert!(stderr.contains("✗ old-a"), "{stderr}");
  assert!(stderr.contains("✗ old-b"), "{stderr}");
  assert!(
    stderr.contains("built for moonlit:plugin@0.3.0, this CLI supports moonlit:plugin@0.4.0"),
    "{stderr}"
  );
  assert!(!stderr.contains("instead"), "{stderr}");
}

#[test]
fn file_plugin_gets_rebuild_hint() {
  let dir = tempfile::tempdir().unwrap();
  let wasm = Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../engine/tests/fixtures/test_plugin_v0_3.wasm")
    .canonicalize()
    .unwrap();
  let yaml = format!(
    "name: demo\nplugins:\n  - name: old\n    url: file://{}\nstages:\n  build:\n    - run: old.log-and-output\n",
    wasm.display()
  );
  std::fs::write(dir.path().join("release.yml"), yaml).unwrap();
  moonlit_in(dir.path())
    .args(["run", "--output", "plain"])
    .assert()
    .code(3)
    .stderr(contains("✗ old  file://"))
    .stderr(contains(
      "built for moonlit:plugin@0.3.0, this CLI supports moonlit:plugin@0.4.0",
    ))
    .stderr(contains("rebuild it against moonlit-pdk for moonlit:plugin@0.4.0"));
}
