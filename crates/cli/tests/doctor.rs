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
    .stdout(contains(
      "✗ test-plugin: not granted: Connect to the internet (api.example.com)\n",
    ))
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

fn oci_entry(server: &MockServer, name: &str, tag: &str) -> String {
  entry(name, &format!("oci://{}/acme/test-plugin:{tag}", server.address()), "")
}

#[tokio::test(flavor = "multi_thread")]
async fn doctor_fixes_two_plugins_in_one_write() {
  let server = registry(&[
    ("0.3.0", "test_plugin_v0_3.wasm", OLD_WORLD),
    ("0.4.0", "pdk_sample.wasm", WORLD),
  ])
  .await;
  let plugins = [
    entry("test-plugin", &file_url("test_plugin.wasm"), ""),
    oci_entry(&server, "sample", "0.3.0"),
  ]
  .concat();
  let project = Project::new(&plugins);

  let output = project.doctor(&["--yes"]).success().get_output().clone();
  let stdout = String::from_utf8_lossy(&output.stdout);
  assert_eq!(stdout.matches("  release.yml\n").count(), 1, "{stdout}");
  assert!(stdout.contains("test-plugin: permissions granted"), "{stdout}");
  assert!(stdout.contains("sample: 0.3.0 → 0.4.0"), "{stdout}");

  let release = project.release();
  assert!(release.contains("network: [\"api.example.com\"]"), "{release}");
  assert!(release.contains("test-plugin:0.4.0"), "{release}");
  project.doctor(&[]).success().stdout(contains("release.yml looks good."));
}

#[tokio::test(flavor = "multi_thread")]
async fn doctor_crosses_majors_only_with_flag() {
  let server = registry(&[
    ("1.0.0", "test_plugin_v0_3.wasm", OLD_WORLD),
    ("2.0.0", "pdk_sample.wasm", WORLD),
  ])
  .await;
  let project = Project::new(&oci_entry(&server, "test-plugin", "1.0.0"));
  let before = project.release();

  project
    .doctor(&[])
    .code(1)
    .stdout(contains("; use 2.0.0 instead (run `moonlit doctor --major` to fix)\n"));
  assert_eq!(project.release(), before);

  project
    .doctor(&["--major"])
    .success()
    .stdout(contains("test-plugin: 1.0.0 → 2.0.0"));
  assert_eq!(project.release(), before.replace(":1.0.0", ":2.0.0"));
}

#[tokio::test(flavor = "multi_thread")]
async fn doctor_move_that_needs_a_grant_asks_for_yes() {
  let server = registry(&[
    ("0.3.0", "test_plugin_v0_3.wasm", OLD_WORLD),
    ("0.4.0", "test_plugin.wasm", WORLD),
  ])
  .await;
  let project = Project::new(&oci_entry(&server, "test-plugin", "0.3.0"));
  let before = project.release();

  project.doctor(&[]).code(2).stderr(contains("pass --yes"));
  assert_eq!(project.release(), before);

  project
    .doctor(&["--yes"])
    .success()
    .stdout(contains("This plugin wants to:"))
    .stdout(contains("test-plugin: 0.3.0 → 0.4.0"));
  let release = project.release();
  assert!(release.contains("test-plugin:0.4.0"), "{release}");
  assert!(release.contains("network: [\"api.example.com\"]"), "{release}");
  assert!(!release.contains("exec:"), "{release}");
}

#[tokio::test(flavor = "multi_thread")]
async fn doctor_exits_3_and_writes_nothing_when_the_registry_is_down() {
  let server = MockServer::start().await;
  Mock::given(method("POST"))
    .and(path("/graphql"))
    .respond_with(ResponseTemplate::new(500))
    .mount(&server)
    .await;
  let wasm = fs::read(fixture_path("test_plugin_v0_3.wasm")).unwrap();
  oci_mock::serve_plugin(&server, "acme/test-plugin", "0.3.0", &wasm, OLD_WORLD).await;
  let plugins = [
    entry("granted", &file_url("test_plugin.wasm"), ""),
    oci_entry(&server, "test-plugin", "0.3.0"),
  ]
  .concat();
  let project = Project::new(&plugins);
  let before = project.release();

  project
    .doctor(&["--yes"])
    .code(3)
    .stdout(contains(
      "✗ granted: not granted: Connect to the internet (api.example.com)\n",
    ))
    .stdout(contains(format!(
      "✗ test-plugin: built for {OLD_WORLD}, this CLI supports {WORLD}; cannot reach the registry\n"
    )));
  assert_eq!(project.release(), before);
}

#[tokio::test(flavor = "multi_thread")]
async fn doctor_reports_a_target_it_cannot_fetch_and_fixes_the_rest() {
  let server = MockServer::start().await;
  let listed: Vec<Value> = ["0.3.0", "0.4.0"]
    .iter()
    .map(|v| json!({ "version": v, "yanked": false, "status": "VERIFIED" }))
    .collect();
  mount(&server, &["query versions("], json!({ "package": { "versions": listed } })).await;
  for (version, world) in [("0.3.0", OLD_WORLD), ("0.4.0", WORLD)] {
    mount(
      &server,
      &["query packageVersion(", &format!("\"version\":\"{version}\"")],
      json!({ "packageVersion": { "version": version, "worldVersion": world, "status": "VERIFIED", "middlewares": [] } }),
    )
    .await;
  }
  let wasm = fs::read(fixture_path("test_plugin_v0_3.wasm")).unwrap();
  oci_mock::serve_plugin(&server, "acme/test-plugin", "0.3.0", &wasm, OLD_WORLD).await;
  let plugins = [
    entry("granted", &file_url("test_plugin.wasm"), ""),
    oci_entry(&server, "test-plugin", "0.3.0"),
  ]
  .concat();
  let project = Project::new(&plugins);
  let before = project.release();

  project
    .doctor(&["--yes"])
    .code(1)
    .stdout(contains(format!(
      "✗ test-plugin: built for {OLD_WORLD}, this CLI supports {WORLD}; cannot fetch oci://{}/acme/test-plugin:0.4.0: ",
      server.address()
    )))
    .stdout(contains("granted: permissions granted"));

  let release = project.release();
  assert!(release.contains("network: [\"api.example.com\"]"), "{release}");
  assert!(release.contains("test-plugin:0.3.0"), "{release}");
  assert_ne!(release, before);
}

#[test]
fn doctor_screen_lists_only_new_permissions() {
  let project = Project::new(&entry(
    "test-plugin",
    &file_url("test_plugin.wasm"),
    "    permissions:\n      exec: [\"git\"]\n",
  ));

  let assert = project
    .doctor(&["--yes"])
    .success()
    .stdout(contains("This plugin wants to:"))
    .stdout(contains("Connect to the internet"));
  let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
  assert!(!stdout.contains("Not requested"), "{stdout}");
}

#[tokio::test(flavor = "multi_thread")]
async fn doctor_gives_the_rebuild_hint_when_the_host_is_not_a_moonlit_registry() {
  let server = MockServer::start().await;
  Mock::given(method("POST"))
    .and(path("/graphql"))
    .respond_with(ResponseTemplate::new(404))
    .mount(&server)
    .await;
  let wasm = fs::read(fixture_path("test_plugin_v0_3.wasm")).unwrap();
  oci_mock::serve_plugin(&server, "acme/test-plugin", "0.3.0", &wasm, OLD_WORLD).await;
  let project = Project::new(&oci_entry(&server, "test-plugin", "0.3.0"));
  let before = project.release();

  project.doctor(&["--yes"]).code(1).stdout(contains(format!(
    "✗ test-plugin: built for {OLD_WORLD}, this CLI supports {WORLD}; rebuild it against moonlit-pdk for {WORLD}\n"
  )));
  assert_eq!(project.release(), before);
}

#[test]
fn doctor_flags_a_component_without_the_plugin_contract() {
  let dir = tempfile::tempdir().unwrap();
  let path = dir.path().join("empty.wasm");
  fs::write(&path, [0x00, 0x61, 0x73, 0x6d, 0x0d, 0x00, 0x01, 0x00]).unwrap();
  let project = Project::new(&entry("test-plugin", &format!("file://{}", path.display()), ""));

  project.doctor(&[]).code(1).stdout(contains(format!(
    "✗ test-plugin: built for an unknown plugin contract, this CLI supports {WORLD}; rebuild it against moonlit-pdk for {WORLD}\n"
  )));
}

#[tokio::test(flavor = "multi_thread")]
async fn doctor_names_the_status_of_a_registry_that_refuses_anonymous_graphql_and_fixes_the_rest() {
  let server = MockServer::start().await;
  Mock::given(method("POST"))
    .and(path("/graphql"))
    .respond_with(ResponseTemplate::new(401))
    .mount(&server)
    .await;
  let wasm = fs::read(fixture_path("test_plugin_v0_3.wasm")).unwrap();
  oci_mock::serve_plugin(&server, "acme/test-plugin", "0.3.0", &wasm, OLD_WORLD).await;
  let plugins = [
    entry("granted", &file_url("test_plugin.wasm"), ""),
    oci_entry(&server, "test-plugin", "0.3.0"),
  ]
  .concat();
  let project = Project::new(&plugins);

  project
    .doctor(&["--yes"])
    .code(1)
    .stdout(contains(format!(
      "registry at {} answered 401 on /graphql, no version to suggest",
      server.address()
    )))
    .stdout(contains("granted: permissions granted"));

  assert!(project.release().contains("network: [\"api.example.com\"]"));
}

#[tokio::test(flavor = "multi_thread")]
async fn doctor_names_the_status_of_a_moved_plugin_in_the_consent_screen() {
  let server = registry(&[
    ("0.3.0", "test_plugin_v0_3.wasm", OLD_WORLD),
    ("0.4.0", "test_plugin.wasm", WORLD),
  ])
  .await;
  let project = Project::new(&oci_entry(&server, "test-plugin", "0.3.0"));

  project
    .doctor(&["--yes"])
    .success()
    .stdout(contains("This plugin wants to:"))
    .stdout(contains("✓ verified"));
}

#[tokio::test(flavor = "multi_thread")]
async fn doctor_leaves_a_digest_pinned_plugin_alone_and_says_how_to_move_it() {
  let server = registry(&[
    ("0.3.0", "test_plugin_v0_3.wasm", OLD_WORLD),
    ("0.4.0", "pdk_sample.wasm", WORLD),
  ])
  .await;
  let wasm = fs::read(fixture_path("test_plugin_v0_3.wasm")).unwrap();
  let digest = oci_mock::serve_plugin(&server, "acme/test-plugin", "0.3.0", &wasm, OLD_WORLD).await;
  let plugins = [
    entry(
      "test-plugin",
      &format!("oci://{}/acme/test-plugin@{digest}", server.address()),
      "",
    ),
    entry("granted", &file_url("test_plugin.wasm"), ""),
  ]
  .concat();
  let project = Project::new(&plugins);
  let before = project.release();

  project
    .doctor(&["--yes"])
    .code(1)
    .stdout(contains(format!("pinned to {digest}")))
    .stdout(contains("change the url to a version tag to move it"))
    .stdout(contains("granted: permissions granted"));

  let release = project.release();
  assert!(release.contains(&format!("@{digest}")), "{release}");
  assert!(release.contains("network: [\"api.example.com\"]"), "{release}");
  assert_ne!(release, before);
}

#[test]
fn doctor_exits_3_when_the_plugin_url_cannot_be_reached() {
  let port = {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
  };
  let plugins = [
    entry("test-plugin", &format!("oci://127.0.0.1:{port}/acme/test-plugin:0.3.0"), ""),
    entry("granted", &file_url("test_plugin.wasm"), ""),
  ]
  .concat();
  let project = Project::new(&plugins);
  let before = project.release();

  project
    .doctor(&["--yes"])
    .code(3)
    .stdout(contains("✗ test-plugin: "))
    .stdout(contains("✗ granted: not granted: "))
    .stderr(contains("cannot reach the registry"));

  assert_eq!(project.release(), before);
}

#[tokio::test(flavor = "multi_thread")]
async fn doctor_exits_3_when_the_move_target_cannot_be_reached() {
  let server = MockServer::start().await;
  let listed: Vec<Value> = ["0.3.0", "0.4.0"]
    .iter()
    .map(|v| json!({ "version": v, "yanked": false, "status": "VERIFIED" }))
    .collect();
  mount(&server, &["query versions("], json!({ "package": { "versions": listed } })).await;
  for (version, world) in [("0.3.0", OLD_WORLD), ("0.4.0", WORLD)] {
    mount(
      &server,
      &["query packageVersion(", &format!("\"version\":\"{version}\"")],
      json!({ "packageVersion": { "version": version, "worldVersion": world, "status": "VERIFIED", "middlewares": [] } }),
    )
    .await;
  }
  let wasm = fs::read(fixture_path("test_plugin_v0_3.wasm")).unwrap();
  oci_mock::serve_plugin(&server, "acme/test-plugin", "0.3.0", &wasm, OLD_WORLD).await;
  Mock::given(method("GET"))
    .and(path("/v2/acme/test-plugin/manifests/0.4.0"))
    .respond_with(ResponseTemplate::new(500))
    .mount(&server)
    .await;
  let plugins = [
    entry("granted", &file_url("test_plugin.wasm"), ""),
    oci_entry(&server, "test-plugin", "0.3.0"),
  ]
  .concat();
  let project = Project::new(&plugins);
  let before = project.release();

  project
    .doctor(&["--yes"])
    .code(3)
    .stdout(contains("✗ granted: not granted: "))
    .stdout(contains(format!(
      "✗ test-plugin: built for {OLD_WORLD}, this CLI supports {WORLD}; cannot fetch oci://{}/acme/test-plugin:0.4.0: ",
      server.address()
    )))
    .stderr(contains("cannot reach the registry"));

  assert_eq!(project.release(), before);
}

fn evil_project(url: &str, permissions: &str) -> Project {
  let project = Project::new(&entry("\"evil\\e[2K\"", url, permissions));
  let release = project
    .release()
    .replace("test-plugin.log-and-output", "\"evil\\e[2K.log-and-output\"");
  fs::write(project.dir.path().join("release.yml"), release).unwrap();
  project
}

fn stdout_of(assert: &assert_cmd::assert::Assert) -> String {
  String::from_utf8(assert.get_output().stdout.clone()).unwrap()
}

#[test]
fn doctor_prints_a_plugin_name_with_escape_codes_as_visible_text() {
  let healthy = evil_project(
    &file_url("test_plugin.wasm"),
    "    permissions:\n      network: [\"api.example.com\"]\n",
  );
  let stdout = stdout_of(&healthy.doctor(&[]).success());
  assert!(!stdout.contains('\u{1b}'), "{stdout:?}");
  assert!(stdout.contains("✓ evil\\x1b[2K"), "{stdout:?}");

  let broken = evil_project(&file_url("does_not_exist.wasm"), "");
  let stdout = stdout_of(&broken.doctor(&[]).code(1));
  assert!(!stdout.contains('\u{1b}'), "{stdout:?}");
  assert!(stdout.contains("✗ evil\\x1b[2K: "), "{stdout:?}");

  let granted = evil_project(&file_url("test_plugin.wasm"), "");
  let stdout = stdout_of(&granted.doctor(&["--yes"]).success());
  assert!(!stdout.contains('\u{1b}'), "{stdout:?}");
  assert!(stdout.contains("evil\\x1b[2K: permissions granted"), "{stdout:?}");
}

#[test]
fn doctor_grant_only_fix_keeps_the_url_line_as_written() {
  let url_line = format!("    url: '{}'  # pinned\n", file_url("test_plugin.wasm"));
  let project = Project::new(&format!("  - name: test-plugin\n{url_line}"));

  project.doctor(&["--yes"]).success();

  let release = project.release();
  assert!(release.contains(&url_line), "{release}");
  assert!(release.contains("network: [\"api.example.com\"]"), "{release}");
}
