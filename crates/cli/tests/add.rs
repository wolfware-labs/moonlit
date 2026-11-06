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
const OLD_WORLD: &str = "moonlit:plugin@0.3.0";
const RELEASE: &str = "# demo pipeline\nname: demo\n\nstages:\n  build:\n    - run: test-plugin.log-and-output\n";

fn fixture(name: &str) -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../engine/tests/fixtures")
    .join(name)
    .canonicalize()
    .expect("fixture exists")
}

fn file_url(name: &str) -> String {
  format!("file://{}", fixture(name).display())
}

struct Project {
  dir: TempDir,
  home: TempDir,
  cache: TempDir,
}

impl Project {
  fn new(release: &str) -> Self {
    let dir = tempfile::tempdir().unwrap();
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

  fn add(&self, args: &[&str]) -> assert_cmd::assert::Assert {
    Command::cargo_bin("moonlit")
      .unwrap()
      .current_dir(self.dir.path())
      .env("MOONLIT_HOME", self.home.path())
      .env("MOONLIT_CACHE_DIR", self.cache.path())
      .args(["--output", "plain", "add"])
      .args(args)
      .assert()
  }
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

async fn registry() -> MockServer {
  let server = MockServer::start().await;
  mount(
    &server,
    &["query versions("],
    json!({ "package": { "versions": [
      { "version": "1.0.0", "yanked": false, "status": "VERIFIED" },
      { "version": "1.1.0", "yanked": false, "status": "VERIFIED" },
      { "version": "2.0.0", "yanked": false, "status": "VERIFIED" }
    ] } }),
  )
  .await;
  for (version, world) in [("1.0.0", WORLD), ("1.1.0", WORLD), ("2.0.0", OLD_WORLD)] {
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
  }
  let plugin = fs::read(fixture("test_plugin.wasm")).unwrap();
  let old = fs::read(fixture("test_plugin_v0_3.wasm")).unwrap();
  oci_mock::serve_plugin(&server, "acme/test-plugin", "1.1.0", &plugin, WORLD).await;
  oci_mock::serve_plugin(&server, "acme/test-plugin", "2.0.0", &old, OLD_WORLD).await;
  server
}

fn host(server: &MockServer) -> String {
  server.address().to_string()
}

#[test]
fn add_file_plugin_with_yes_writes_required_grants() {
  let project = Project::new(RELEASE);
  let url = file_url("test_plugin.wasm");

  let assert = project
    .add(&[&url, "--yes"])
    .success()
    .stdout(contains("This plugin wants to:"))
    .stdout(contains("Connect to the internet"))
    .stdout(contains("\"Calls the example API.\""))
    .stdout(contains("\n  release.yml\n  + plugins:\n"))
    .stdout(contains("Added test-plugin to release.yml."));
  let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
  assert!(stdout.find("Not requested:") < stdout.find("+ plugins:"), "{stdout}");

  let release = project.release();
  assert!(release.starts_with(RELEASE.split("stages:").next().unwrap()), "{release}");
  assert!(release.contains("  - name: test-plugin\n"), "{release}");
  assert!(release.contains(&format!("    url: {url}\n")), "{release}");
  assert!(release.contains("      network: [\"api.example.com\"]\n"), "{release}");
  assert!(!release.contains("exec:"), "{release}");
  assert!(
    release.ends_with("stages:\n  build:\n    - run: test-plugin.log-and-output\n"),
    "{release}"
  );
}

#[tokio::test(flavor = "multi_thread")]
async fn yes_grants_required_only() {
  let server = registry().await;
  let project = Project::new(RELEASE);

  project
    .add(&["acme/test-plugin", "--yes", "--registry", &server.uri()])
    .success()
    .stdout(contains("test-plugin 1.1.0"))
    .stdout(contains("Optional:"))
    .stdout(contains("Added test-plugin to release.yml."));

  let release = project.release();
  let url = format!("oci://{}/acme/test-plugin:1.1.0", host(&server));
  assert!(release.contains(&url), "{release}");
  assert!(release.contains("network: [\"api.example.com\"]"), "{release}");
  assert!(!release.contains("exec:"), "{release}");
}

#[tokio::test(flavor = "multi_thread")]
async fn with_optional_grants_all() {
  let server = registry().await;
  let project = Project::new(RELEASE);

  project
    .add(&[
      "acme/test-plugin@1.1.0",
      "--yes",
      "--with-optional",
      "--registry",
      &server.uri(),
    ])
    .success();

  let release = project.release();
  assert!(release.contains("network: [\"api.example.com\"]"), "{release}");
  assert!(release.contains("exec: [\"git\"]"), "{release}");
}

#[tokio::test(flavor = "multi_thread")]
async fn pinned_version_for_another_world_fails() {
  let server = registry().await;
  let project = Project::new(RELEASE);

  project
    .add(&["acme/test-plugin@2.0.0", "--yes", "--registry", &server.uri()])
    .code(3)
    .stderr(contains(OLD_WORLD))
    .stderr(contains(WORLD));

  assert_eq!(project.release(), RELEASE);
}

#[test]
fn no_tty_without_yes_refuses() {
  let project = Project::new(RELEASE);

  project.add(&[&file_url("test_plugin.wasm")]).code(2).stderr(contains(
    "error: refusing to grant permissions without a terminal; pass --yes to accept the plugin's required permissions",
  ));

  assert_eq!(project.release(), RELEASE);
}

#[test]
fn add_old_world_plugin_fails() {
  let project = Project::new(RELEASE);

  project
    .add(&[&file_url("test_plugin_v0_3.wasm"), "--yes"])
    .code(3)
    .stderr(contains(OLD_WORLD))
    .stderr(contains(WORLD));

  assert_eq!(project.release(), RELEASE);
}

#[test]
fn add_name_collision_suggests_name_flag() {
  let release = format!(
    "name: demo\nplugins:\n  - name: test-plugin\n    url: {}\nstages:\n  build:\n    - run: test-plugin.echo\n",
    file_url("pdk_sample.wasm")
  );
  let project = Project::new(&release);
  let url = file_url("test_plugin.wasm");

  project
    .add(&[&url, "--yes"])
    .code(2)
    .stderr(contains("a plugin named 'test-plugin' is already declared"))
    .stderr(contains("--name"));
  assert_eq!(project.release(), release);

  project
    .add(&[&url, "--yes", "--name", "tp"])
    .success()
    .stdout(contains("Added tp to release.yml."));
  assert!(project.release().contains("  - name: tp\n"));
}
