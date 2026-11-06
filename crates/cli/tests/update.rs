#[path = "support/oci_mock.rs"]
mod oci_mock;

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::str::contains;
use serde_json::{Value, json};
use tempfile::TempDir;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const WORLD: &str = "moonlit:plugin@0.4.0";

fn fixture(name: &str) -> Vec<u8> {
  let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../engine/tests/fixtures")
    .join(name);
  fs::read(path).expect("fixture exists")
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

async fn registry(versions: &[(&str, &str)]) -> MockServer {
  let server = MockServer::start().await;
  let listed: Vec<Value> = versions
    .iter()
    .map(|(v, _)| json!({ "version": v, "yanked": false, "status": "VERIFIED" }))
    .collect();
  mount(&server, &["query versions("], json!({ "package": { "versions": listed } })).await;
  for (version, fixture_name) in versions {
    mount(
      &server,
      &["query packageVersion(", &format!("\"version\":\"{version}\"")],
      json!({ "packageVersion": {
        "version": version,
        "worldVersion": WORLD,
        "status": "VERIFIED",
        "middlewares": []
      } }),
    )
    .await;
    oci_mock::serve_plugin(&server, "acme/test-plugin", version, &fixture(fixture_name), WORLD).await;
  }
  server
}

struct Project {
  dir: TempDir,
  home: TempDir,
  cache: TempDir,
}

impl Project {
  fn new(server: &MockServer, current: &str, permissions: &str) -> Self {
    let dir = tempfile::tempdir().unwrap();
    let release = format!(
      "name: demo\n\nplugins:\n  - name: test-plugin\n    url: oci://{}/acme/test-plugin:{current}\n{permissions}\nstages:\n  build:\n    - run: test-plugin.log-and-output\n",
      server.address()
    );
    fs::write(dir.path().join("release.yml"), release).unwrap();
    Self {
      dir,
      home: tempfile::tempdir().unwrap(),
      cache: tempfile::tempdir().unwrap(),
    }
  }

  fn release(&self) -> String {
    fs::read_to_string(self.dir.path().join("release.yml")).unwrap()
  }

  fn update(&self, args: &[&str]) -> assert_cmd::assert::Assert {
    Command::cargo_bin("moonlit")
      .unwrap()
      .current_dir(self.dir.path())
      .env("MOONLIT_HOME", self.home.path())
      .env("MOONLIT_CACHE_DIR", self.cache.path())
      .args(["--output", "plain", "update"])
      .args(args)
      .assert()
  }
}

#[tokio::test(flavor = "multi_thread")]
async fn update_bumps_without_prompt_when_no_new_required() {
  let server = registry(&[("1.0.0", "pdk_sample.wasm"), ("1.0.1", "pdk_sample.wasm")]).await;
  let project = Project::new(&server, "1.0.0", "");

  project.update(&[]).success().stdout(contains("test-plugin: 1.0.0 → 1.0.1"));

  let release = project.release();
  assert!(release.contains("test-plugin:1.0.1"), "{release}");
  assert!(!release.contains("network:") && !release.contains("exec:"), "{release}");
}

#[tokio::test(flavor = "multi_thread")]
async fn update_reports_up_to_date() {
  let server = registry(&[("1.0.0", "pdk_sample.wasm")]).await;
  let project = Project::new(&server, "1.0.0", "");

  project.update(&[]).success().stdout(contains("test-plugin: up to date"));
}

#[tokio::test(flavor = "multi_thread")]
async fn update_asks_when_new_required_and_yes_grants_it() {
  let server = registry(&[("1.0.0", "pdk_sample.wasm"), ("1.1.0", "test_plugin.wasm")]).await;
  let project = Project::new(&server, "1.0.0", "");
  let before = project.release();

  project.update(&[]).code(2).stderr(contains("pass --yes"));
  assert_eq!(project.release(), before);

  project
    .update(&["--yes"])
    .success()
    .stdout(contains("This plugin wants to:"))
    .stdout(contains("Connect to the internet"))
    .stdout(contains("test-plugin: 1.0.0 → 1.1.0"));

  let release = project.release();
  assert!(release.contains("test-plugin:1.1.0"), "{release}");
  assert!(release.contains("network: [\"api.example.com\"]"), "{release}");
  assert!(!release.contains("exec:"), "{release}");
}

#[tokio::test(flavor = "multi_thread")]
async fn update_lists_new_optional_without_granting() {
  let server = registry(&[("1.0.0", "pdk_sample.wasm"), ("1.1.0", "test_plugin.wasm")]).await;
  let project = Project::new(&server, "1.0.0", "    permissions:\n      network: [\"api.example.com\"]\n");

  project
    .update(&[])
    .success()
    .stdout(contains(
      "test-plugin: new optional permission available: Run programs on your machine (git)",
    ))
    .stdout(contains("test-plugin: 1.0.0 → 1.1.0"));

  let release = project.release();
  assert!(release.contains("test-plugin:1.1.0"), "{release}");
  assert!(!release.contains("exec:"), "{release}");
}

#[tokio::test(flavor = "multi_thread")]
async fn update_stays_in_major_without_flag() {
  let server = registry(&[
    ("1.0.0", "pdk_sample.wasm"),
    ("1.0.1", "pdk_sample.wasm"),
    ("2.0.0", "pdk_sample.wasm"),
  ])
  .await;
  let project = Project::new(&server, "1.0.0", "");

  project.update(&[]).success().stdout(contains("test-plugin: 1.0.0 → 1.0.1"));
  assert!(project.release().contains("test-plugin:1.0.1"));

  project
    .update(&["--major"])
    .success()
    .stdout(contains("test-plugin: 1.0.1 → 2.0.0"));
  assert!(project.release().contains("test-plugin:2.0.0"));
}
