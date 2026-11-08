#[path = "support/oci_mock.rs"]
mod oci_mock;
#[cfg(unix)]
#[path = "support/terminal.rs"]
mod terminal;

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
  let with_world: Vec<(&str, &str, &str)> = versions.iter().map(|(v, f)| (*v, *f, WORLD)).collect();
  registry_with_worlds(&with_world).await
}

async fn registry_with_worlds(versions: &[(&str, &str, &str)]) -> MockServer {
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
    oci_mock::serve_plugin(&server, "acme/test-plugin", version, &fixture(fixture_name), world).await;
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

  #[cfg(unix)]
  fn update_on_terminal(&self, args: &[&str], keys: &[u8]) -> std::process::Output {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_moonlit"));
    command
      .current_dir(self.dir.path())
      .env("MOONLIT_HOME", self.home.path())
      .env("MOONLIT_CACHE_DIR", self.cache.path())
      .args(["--output", "plain", "update"])
      .args(args);
    terminal::run_on_terminal(command, keys)
  }
}

#[tokio::test(flavor = "multi_thread")]
async fn update_bumps_without_prompt_when_no_new_required() {
  let server = registry(&[("1.0.0", "pdk_sample.wasm"), ("1.0.1", "pdk_sample.wasm")]).await;
  let project = Project::new(&server, "1.0.0", "");
  let before = project.release();

  project.update(&[]).success().stdout(contains("test-plugin: 1.0.0 → 1.0.1"));

  assert_eq!(project.release(), before.replace(":1.0.0", ":1.0.1"));
}

#[tokio::test(flavor = "multi_thread")]
async fn update_reports_up_to_date() {
  let server = registry(&[("1.0.0", "pdk_sample.wasm")]).await;
  let project = Project::new(&server, "1.0.0", "");

  project.update(&[]).success().stdout(contains("test-plugin: up to date"));
}

#[tokio::test(flavor = "multi_thread")]
async fn update_says_so_when_no_newer_version_is_compatible() {
  let server = registry_with_worlds(&[
    ("1.0.0", "test_plugin_v0_3.wasm", OLD_WORLD),
    ("1.0.1", "test_plugin_v0_3.wasm", OLD_WORLD),
  ])
  .await;
  let project = Project::new(&server, "1.0.0", "");
  let before = project.release();

  let assert = project
    .update(&[])
    .success()
    .stdout(contains("test-plugin: no newer compatible version"));
  let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
  assert!(!stdout.contains("up to date"), "{stdout}");
  assert_eq!(project.release(), before);
}

#[tokio::test(flavor = "multi_thread")]
async fn update_keeps_saying_up_to_date_when_the_current_version_is_the_newest_compatible() {
  let server = registry_with_worlds(&[
    ("1.0.0", "pdk_sample.wasm", WORLD),
    ("1.0.1", "test_plugin_v0_3.wasm", OLD_WORLD),
  ])
  .await;
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

  let assert = project
    .update(&["--yes"])
    .success()
    .stdout(contains("This plugin wants to:"))
    .stdout(contains("Connect to the internet"))
    .stdout(contains("test-plugin: 1.0.0 → 1.1.0"));
  let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
  assert!(!stdout.contains('\u{1b}'), "{stdout:?}");
  assert!(!stdout.contains("optional permission not granted: Connect"), "{stdout}");

  let release = project.release();
  assert!(release.contains("test-plugin:1.1.0"), "{release}");
  assert!(release.contains("network: [\"api.example.com\"]"), "{release}");
  assert!(!release.contains("exec:"), "{release}");
}

#[tokio::test(flavor = "multi_thread")]
async fn update_lists_new_optional_without_granting() {
  let server = registry(&[("1.0.0", "pdk_sample.wasm"), ("1.1.0", "test_plugin.wasm")]).await;
  let project = Project::new(
    &server,
    "1.0.0",
    "    permissions:   # hand written\n      network: [ \"api.example.com\" ]\n",
  );
  let before = project.release();

  project
    .update(&[])
    .success()
    .stdout(contains(
      "test-plugin: optional permission not granted: Run programs on your machine (git)",
    ))
    .stdout(contains("test-plugin: 1.0.0 → 1.1.0"));

  assert_eq!(project.release(), before.replace(":1.0.0", ":1.1.0"));
}

#[tokio::test(flavor = "multi_thread")]
async fn update_says_when_a_tag_is_not_a_version() {
  let server = registry(&[("1.0.0", "pdk_sample.wasm")]).await;
  let project = Project::new(&server, "latest", "");
  let before = project.release();

  project
    .update(&[])
    .success()
    .stdout(contains("test-plugin: pinned to latest, skipped"));
  project
    .update(&["test-plugin"])
    .success()
    .stdout(contains("test-plugin: pinned to latest, skipped"));
  assert_eq!(project.release(), before);
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

#[tokio::test(flavor = "multi_thread")]
async fn update_screen_lists_only_new_permissions() {
  let server = registry(&[("1.0.0", "pdk_sample.wasm"), ("1.1.0", "test_plugin.wasm")]).await;
  let project = Project::new(&server, "1.0.0", "    permissions:\n      exec: [\"git\"]\n");

  let assert = project
    .update(&["--yes"])
    .success()
    .stdout(contains("This plugin wants to:"))
    .stdout(contains("Connect to the internet"));
  let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
  assert!(!stdout.contains("Not requested"), "{stdout}");
}

#[tokio::test(flavor = "multi_thread")]
async fn update_skips_a_plugin_whose_host_is_not_a_moonlit_registry() {
  let server = registry(&[("1.0.0", "pdk_sample.wasm"), ("1.0.1", "pdk_sample.wasm")]).await;
  let plain_oci = MockServer::start().await;
  Mock::given(method("POST"))
    .and(path("/graphql"))
    .respond_with(ResponseTemplate::new(404))
    .mount(&plain_oci)
    .await;
  let project = Project::new(&server, "1.0.0", "");
  let release = project.release().replace(
    "\nstages:",
    &format!(
      "  - name: other\n    url: oci://{}/acme/other:1.0.0\n\nstages:",
      plain_oci.address()
    ),
  );
  fs::write(project.dir.path().join("release.yml"), &release).unwrap();

  project
    .update(&[])
    .success()
    .stdout(contains("other: not from a Moonlit registry, skipped"))
    .stdout(contains("test-plugin: 1.0.0 → 1.0.1"));

  assert_eq!(project.release(), release.replace("test-plugin:1.0.0", "test-plugin:1.0.1"));
}

#[tokio::test(flavor = "multi_thread")]
async fn update_fails_a_registry_that_refuses_anonymous_graphql_and_updates_the_rest() {
  let server = registry(&[("1.0.0", "pdk_sample.wasm"), ("1.0.1", "pdk_sample.wasm")]).await;
  for status in [401, 403] {
    let guarded = MockServer::start().await;
    Mock::given(method("POST"))
      .and(path("/graphql"))
      .respond_with(ResponseTemplate::new(status))
      .mount(&guarded)
      .await;
    let project = Project::new(&server, "1.0.0", "");
    let release = with_other(&project, &guarded);

    project
      .update(&[])
      .code(1)
      .stdout(contains(format!(
        "other: registry at {} answered {status} on /graphql, skipped",
        guarded.address()
      )))
      .stdout(contains("test-plugin: 1.0.0 → 1.0.1"));

    assert_eq!(project.release(), release.replace("test-plugin:1.0.0", "test-plugin:1.0.1"));
  }
}

fn evil_name(project: &Project) {
  let release = project
    .release()
    .replace("name: test-plugin", "name: \"evil\\e[2K\"")
    .replace("test-plugin.log-and-output", "\"evil\\e[2K.log-and-output\"");
  fs::write(project.dir.path().join("release.yml"), release).unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn update_prints_a_plugin_name_with_escape_codes_as_visible_text() {
  let server = registry(&[("1.0.0", "pdk_sample.wasm"), ("1.0.1", "pdk_sample.wasm")]).await;

  let bumped = Project::new(&server, "1.0.0", "");
  evil_name(&bumped);
  let assert = bumped.update(&[]).success();
  let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
  assert!(!stdout.contains('\u{1b}'), "{stdout:?}");
  assert!(stdout.contains("evil\\x1b[2K: 1.0.0 → 1.0.1"), "{stdout:?}");

  let pinned = Project::new(&server, "latest", "");
  evil_name(&pinned);
  let assert = pinned.update(&[]).success();
  let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
  assert!(!stdout.contains('\u{1b}'), "{stdout:?}");
  assert!(stdout.contains("evil\\x1b[2K: pinned to latest, skipped"), "{stdout:?}");
}

fn with_other(project: &Project, server: &MockServer) -> String {
  let release = project.release().replace(
    "\nstages:",
    &format!(
      "  - name: other\n    url: oci://{}/acme/other:1.0.0\n\nstages:",
      server.address()
    ),
  );
  fs::write(project.dir.path().join("release.yml"), &release).unwrap();
  release
}

#[tokio::test(flavor = "multi_thread")]
async fn update_skips_a_plugin_whose_lookup_fails_and_updates_the_rest() {
  let server = registry(&[("1.0.0", "pdk_sample.wasm"), ("1.0.1", "pdk_sample.wasm")]).await;
  let failing = MockServer::start().await;
  Mock::given(method("POST"))
    .and(path("/graphql"))
    .respond_with(ResponseTemplate::new(500))
    .mount(&failing)
    .await;
  let project = Project::new(&server, "1.0.0", "");
  let release = with_other(&project, &failing);

  project
    .update(&[])
    .code(1)
    .stdout(contains("other: registry request failed: "))
    .stdout(contains(", skipped\n"))
    .stdout(contains("test-plugin: 1.0.0 → 1.0.1"));

  assert_eq!(project.release(), release.replace("test-plugin:1.0.0", "test-plugin:1.0.1"));
}

#[tokio::test(flavor = "multi_thread")]
async fn update_skips_a_plugin_whose_new_version_cannot_be_fetched_and_updates_the_rest() {
  let server = registry(&[("1.0.0", "pdk_sample.wasm"), ("1.0.1", "pdk_sample.wasm")]).await;
  let missing = MockServer::start().await;
  mount(
    &missing,
    &["query versions("],
    json!({ "package": { "versions": [
      { "version": "1.0.0", "yanked": false, "status": "VERIFIED" },
      { "version": "1.0.1", "yanked": false, "status": "VERIFIED" }
    ] } }),
  )
  .await;
  mount(
    &missing,
    &["query packageVersion("],
    json!({ "packageVersion": { "version": "1.0.1", "worldVersion": WORLD, "status": "VERIFIED", "middlewares": [] } }),
  )
  .await;
  let project = Project::new(&server, "1.0.0", "");
  let release = with_other(&project, &missing);

  project
    .update(&[])
    .code(1)
    .stdout(contains(format!(
      "other: cannot resolve 'oci://{}/acme/other:1.0.1'",
      missing.address()
    )))
    .stdout(contains(", skipped\n"))
    .stdout(contains("test-plugin: 1.0.0 → 1.0.1"));

  assert_eq!(project.release(), release.replace("test-plugin:1.0.0", "test-plugin:1.0.1"));
}

#[tokio::test(flavor = "multi_thread")]
async fn update_exits_3_and_writes_nothing_when_a_registry_cannot_be_reached() {
  let server = registry(&[("1.0.0", "pdk_sample.wasm"), ("1.0.1", "pdk_sample.wasm")]).await;
  let port = {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
  };
  let project = Project::new(&server, "1.0.0", "");
  let release = project.release().replace(
    "\nstages:",
    &format!("  - name: other\n    url: oci://127.0.0.1:{port}/acme/other:1.0.0\n\nstages:"),
  );
  fs::write(project.dir.path().join("release.yml"), &release).unwrap();

  project
    .update(&[])
    .code(3)
    .stderr(contains("other: registry request failed: "));

  assert_eq!(project.release(), release);
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn update_on_a_terminal_keeps_the_version_when_the_new_grant_is_refused() {
  let server = registry(&[("1.0.0", "pdk_sample.wasm"), ("1.1.0", "test_plugin.wasm")]).await;
  let project = Project::new(&server, "1.0.0", "");
  let before = project.release();

  let output = project.update_on_terminal(&[], b"n");
  let stdout = String::from_utf8(output.stdout).unwrap();
  assert_eq!(output.status.code(), Some(1), "{stdout}");
  assert!(stdout.contains("test-plugin: kept at 1.0.0"), "{stdout}");
  assert_eq!(project.release(), before);

  let output = project.update_on_terminal(&[], b"y");
  let stdout = String::from_utf8(output.stdout).unwrap();
  assert_eq!(output.status.code(), Some(0), "{stdout}");
  assert!(stdout.contains("test-plugin: 1.0.0 → 1.1.0"), "{stdout}");
  assert!(project.release().contains("test-plugin:1.1.0"));
}
