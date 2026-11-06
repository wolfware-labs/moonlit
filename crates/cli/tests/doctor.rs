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

const OLD_WORLD: &str = "moonlit:plugin@0.3.0";
const WORLD: &str = "moonlit:plugin@0.4.0";

fn fixture_path(name: &str) -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../engine/tests/fixtures")
    .join(name)
}

fn file_url(name: &str) -> String {
  format!("file://{}", fixture_path(name).display())
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

async fn registry(versions: &[(&str, &str, &str)]) -> MockServer {
  let server = MockServer::start().await;
  let listed: Vec<Value> = versions
    .iter()
    .map(|(v, _, _)| json!({ "version": v, "yanked": false, "status": "VERIFIED" }))
    .collect();
  mount(&server, &["query versions("], json!({ "package": { "versions": listed } })).await;
  for (version, fixture_name, world) in versions {
    mount(
      &server,
      &["query packageVersion(", &format!("\"version\":\"{version}\"")],
      json!({ "packageVersion": {
        "version": version,
        "worldVersion": world,
        "status": "VERIFIED",
        "middlewares": []
      } }),
    )
    .await;
    let wasm = fs::read(fixture_path(fixture_name)).unwrap();
    oci_mock::serve_plugin(&server, "acme/test-plugin", version, &wasm, world).await;
  }
  server
}

struct Project {
  dir: TempDir,
  home: TempDir,
  cache: TempDir,
}

impl Project {
  fn new(plugins: &str) -> Self {
    let dir = tempfile::tempdir().unwrap();
    let release = format!("name: demo\n\nplugins:\n{plugins}\nstages:\n  build:\n    - run: test-plugin.log-and-output\n");
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

  fn doctor(&self, args: &[&str]) -> assert_cmd::assert::Assert {
    Command::cargo_bin("moonlit")
      .unwrap()
      .current_dir(self.dir.path())
      .env("MOONLIT_HOME", self.home.path())
      .env("MOONLIT_CACHE_DIR", self.cache.path())
      .args(["--output", "plain", "doctor"])
      .args(args)
      .assert()
  }
}

fn entry(name: &str, url: &str, permissions: &str) -> String {
  format!("  - name: {name}\n    url: {url}\n{permissions}")
}

#[test]
fn doctor_all_clear() {
  let project = Project::new(&entry(
    "test-plugin",
    &file_url("test_plugin.wasm"),
    "    permissions:\n      network: [\"api.example.com\"]\n",
  ));
  let before = project.release();

  project
    .doctor(&[])
    .success()
    .stdout(contains("✓ test-plugin"))
    .stdout(contains("release.yml looks good."));

  assert_eq!(project.release(), before);
}

#[test]
fn doctor_fixes_missing_grant_with_yes() {
  let project = Project::new(&entry("test-plugin", &file_url("test_plugin.wasm"), ""));
  let before = project.release();

  project
    .doctor(&[])
    .code(2)
    .stdout(contains("✗ test-plugin: "))
    .stderr(contains("pass --yes"));
  assert_eq!(project.release(), before);

  project
    .doctor(&["--yes"])
    .success()
    .stdout(contains("This plugin wants to:"))
    .stdout(contains("Connect to the internet"));

  let release = project.release();
  assert!(release.contains("network: [\"api.example.com\"]"), "{release}");
  assert!(!release.contains("exec:"), "{release}");
  project.doctor(&[]).success().stdout(contains("release.yml looks good."));
}

#[tokio::test(flavor = "multi_thread")]
async fn doctor_moves_incompatible_plugin() {
  let server = registry(&[
    ("0.3.0", "test_plugin_v0_3.wasm", OLD_WORLD),
    ("0.4.0", "pdk_sample.wasm", WORLD),
  ])
  .await;
  let project = Project::new(&entry(
    "test-plugin",
    &format!("oci://{}/acme/test-plugin:0.3.0", server.address()),
    "",
  ));
  let before = project.release();

  project
    .doctor(&[])
    .success()
    .stdout(contains(format!(
      "✗ test-plugin: built for {OLD_WORLD}, this CLI supports {WORLD}"
    )))
    .stdout(contains("test-plugin: 0.3.0 → 0.4.0"));

  assert_eq!(project.release(), before.replace(":0.3.0", ":0.4.0"));
}

#[test]
fn doctor_reports_unfixable_exit_1() {
  let plugins = [
    entry("test-plugin", &file_url("test_plugin_v0_3.wasm"), ""),
    entry("gone", &file_url("does_not_exist.wasm"), ""),
  ]
  .concat();
  let project = Project::new(&plugins);
  let before = project.release();

  project
    .doctor(&[])
    .code(1)
    .stdout(contains(format!(
      "✗ test-plugin: built for {OLD_WORLD}, this CLI supports {WORLD}"
    )))
    .stdout(contains("rebuild it against moonlit-pdk"))
    .stdout(contains("✗ gone: cannot resolve"));

  assert_eq!(project.release(), before);
}
