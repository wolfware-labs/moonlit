use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
use serde_json::{Value, json};
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn mount(server: &MockServer, marker: &str, data: Value) {
  Mock::given(method("POST"))
    .and(path("/graphql"))
    .and(body_string_contains(marker))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": data })))
    .mount(server)
    .await;
}

async fn registry_with_github() -> MockServer {
  let server = MockServer::start().await;
  mount(
    &server,
    "packages(",
    json!({ "packages": { "items": [
      { "orgSlug": "wolfware", "name": "github", "description": "GitHub releases",
        "latestVersion": "2.0.0", "status": "VERIFIED", "middlewares": ["create-release"] }
    ], "cursor": null } }),
  )
  .await;
  mount(
    &server,
    "packageVersion",
    json!({ "packageVersion": {
      "version": "2.0.0",
      "worldVersion": "moonlit:plugin@0.4.0",
      "status": "VERIFIED",
      "middlewares": [{ "name": "create-release", "description": "Creates a GitHub release" }]
    } }),
  )
  .await;
  server
}

fn search(server: &MockServer, query: &str, output: &str) -> assert_cmd::assert::Assert {
  Command::cargo_bin("moonlit")
    .unwrap()
    .args(["search", query, "--registry", &server.uri(), "--output", output])
    .assert()
}

#[tokio::test(flavor = "multi_thread")]
async fn search_prints_cards_with_middleware_tables() {
  let server = registry_with_github().await;

  search(&server, "github", "plain")
    .success()
    .stdout(contains("1 plugin\n"))
    .stdout(contains("wolfware/github 2.0.0"))
    .stdout(contains("✓ verified"))
    .stdout(contains("  ┌────────────────┬─"))
    .stdout(contains("  │ MIDDLEWARE     │ DESCRIPTION"))
    .stdout(contains("  ├────────────────┼─"))
    .stdout(contains("  │ create-release │ Creates a GitHub release"))
    .stdout(contains("  └────────────────┴─"))
    .stdout(contains("create-release"))
    .stdout(contains("Creates a GitHub release"))
    .stdout(contains("GitHub releases"))
    .stdout(contains("Add one: moonlit add wolfware/github"));
}

#[tokio::test(flavor = "multi_thread")]
async fn search_json_lists_hits_with_middlewares() {
  let server = registry_with_github().await;

  let out = search(&server, "github", "json").success().get_output().stdout.clone();
  let parsed: Value = serde_json::from_slice(&out).unwrap();

  assert_eq!(parsed[0]["org"], "wolfware");
  assert_eq!(parsed[0]["version"], "2.0.0");
  assert_eq!(parsed[0]["status"], "verified");
  assert_eq!(parsed[0]["compatible"], true);
  assert_eq!(parsed[0]["middlewares"][0]["name"], "create-release");
}

async fn registry_with_latest_on_world(world: &str) -> MockServer {
  let server = MockServer::start().await;
  mount(
    &server,
    "packages(",
    json!({ "packages": { "items": [
      { "orgSlug": "wolfware", "name": "github", "description": "GitHub releases",
        "latestVersion": "3.0.0", "status": "VERIFIED", "middlewares": ["create-release"] }
    ], "cursor": null } }),
  )
  .await;
  mount(
    &server,
    "packageVersion",
    json!({ "packageVersion": {
      "version": "3.0.0",
      "worldVersion": world,
      "status": "VERIFIED",
      "middlewares": [{ "name": "create-release", "description": "Creates a GitHub release" }]
    } }),
  )
  .await;
  server
}

#[tokio::test(flavor = "multi_thread")]
async fn search_marks_a_latest_version_built_for_another_world() {
  let server = registry_with_latest_on_world("moonlit:plugin@0.3.0").await;

  search(&server, "github", "plain").success().stdout(contains(
    "wolfware/github 3.0.0 (not compatible with this moonlit; add picks the newest compatible version)",
  ));

  let out = search(&server, "github", "json").success().get_output().stdout.clone();
  let parsed: Value = serde_json::from_slice(&out).unwrap();
  assert_eq!(parsed[0]["version"], "3.0.0");
  assert_eq!(parsed[0]["compatible"], false);
}

#[tokio::test(flavor = "multi_thread")]
async fn search_does_not_mark_a_compatible_latest_version() {
  let server = registry_with_github().await;

  search(&server, "github", "plain")
    .success()
    .stdout(contains("not compatible").not());
}

#[tokio::test(flavor = "multi_thread")]
async fn search_marks_a_latest_version_without_a_world() {
  let server = MockServer::start().await;
  mount(
    &server,
    "packages(",
    json!({ "packages": { "items": [
      { "orgSlug": "wolfware", "name": "github", "description": null,
        "latestVersion": "1.0.0", "status": "VERIFIED", "middlewares": [] }
    ], "cursor": null } }),
  )
  .await;
  mount(
    &server,
    "packageVersion",
    json!({ "packageVersion": { "version": "1.0.0", "worldVersion": null, "status": "VERIFIED", "middlewares": [] } }),
  )
  .await;

  search(&server, "github", "plain").success().stdout(contains(
    "wolfware/github 1.0.0 (not compatible with this moonlit; add picks the newest compatible version)",
  ));
}

#[tokio::test(flavor = "multi_thread")]
async fn search_prints_the_bare_id_for_a_hit_without_a_latest_version() {
  let server = MockServer::start().await;
  mount(
    &server,
    "packages(",
    json!({ "packages": { "items": [
      { "orgSlug": "wolfware", "name": "draft", "description": null,
        "latestVersion": null, "status": "PENDING", "middlewares": [] }
    ], "cursor": null } }),
  )
  .await;

  let assert = search(&server, "draft", "plain").success();
  let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
  assert!(stdout.contains("wolfware/draft   pending"), "{stdout}");
  assert!(!stdout.contains("MIDDLEWARE"), "{stdout}");
  assert!(!stdout.contains("not compatible"), "{stdout}");

  let out = search(&server, "draft", "json").success().get_output().stdout.clone();
  let parsed: Value = serde_json::from_slice(&out).unwrap();
  assert_eq!(parsed[0]["version"], Value::Null);
  assert!(parsed[0].as_object().unwrap().contains_key("compatible"));
  assert_eq!(parsed[0]["compatible"], Value::Null);
}

#[tokio::test(flavor = "multi_thread")]
async fn search_keeps_the_hit_middlewares_when_there_are_no_version_details() {
  let server = MockServer::start().await;
  mount(
    &server,
    "packages(",
    json!({ "packages": { "items": [
      { "orgSlug": "wolfware", "name": "draft", "description": null,
        "latestVersion": null, "status": "PENDING", "middlewares": ["create-release"] }
    ], "cursor": null } }),
  )
  .await;

  search(&server, "draft", "plain")
    .success()
    .stdout(contains("  │ MIDDLEWARE     │ DESCRIPTION"))
    .stdout(contains("  │ create-release │"));

  let out = search(&server, "draft", "json").success().get_output().stdout.clone();
  let parsed: Value = serde_json::from_slice(&out).unwrap();
  assert_eq!(parsed[0]["middlewares"][0]["name"], "create-release");
  assert_eq!(parsed[0]["middlewares"][0]["description"], Value::Null);
}

#[tokio::test(flavor = "multi_thread")]
async fn search_fetches_hit_details_concurrently_and_keeps_the_hit_order() {
  let server = MockServer::start().await;
  let names: Vec<String> = (0..12).map(|i| format!("p{i:02}")).collect();
  let items: Vec<Value> = names
    .iter()
    .map(|n| json!({ "orgSlug": "acme", "name": n, "description": null, "latestVersion": "1.0.0", "status": "VERIFIED" }))
    .collect();
  mount(
    &server,
    "packages(",
    json!({ "packages": { "items": items, "cursor": null } }),
  )
  .await;
  for (i, name) in names.iter().enumerate() {
    let delay = if i == 0 { 900 } else { 600 };
    Mock::given(method("POST"))
      .and(path("/graphql"))
      .and(body_string_contains("packageVersion"))
      .and(body_string_contains(format!("\"name\":\"{name}\"")))
      .respond_with(
        ResponseTemplate::new(200)
          .set_delay(std::time::Duration::from_millis(delay))
          .set_body_json(json!({ "data": { "packageVersion": {
            "version": "1.0.0", "worldVersion": "moonlit:plugin@0.4.0", "status": "VERIFIED",
            "middlewares": [{ "name": format!("mw-{name}"), "description": null }]
          } } })),
      )
      .mount(&server)
      .await;
  }

  let started = std::time::Instant::now();
  let out = search(&server, "p", "json").success().get_output().stdout.clone();
  let elapsed = started.elapsed();
  let parsed: Value = serde_json::from_slice(&out).unwrap();

  let order: Vec<&str> = parsed
    .as_array()
    .unwrap()
    .iter()
    .map(|c| c["name"].as_str().unwrap())
    .collect();
  assert_eq!(order, names.iter().map(String::as_str).collect::<Vec<_>>());
  assert_eq!(parsed[0]["middlewares"][0]["name"], "mw-p00");
  assert_eq!(parsed[11]["middlewares"][0]["name"], "mw-p11");
  assert!(elapsed < std::time::Duration::from_secs(4), "took {elapsed:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn search_empty_result() {
  let server = MockServer::start().await;
  mount(&server, "packages(", json!({ "packages": { "items": [], "cursor": null } })).await;

  search(&server, "zzz", "plain")
    .success()
    .stdout(contains("No plugins match 'zzz'."));
}

#[tokio::test(flavor = "multi_thread")]
async fn search_registry_down_exits_3() {
  let server = MockServer::start().await;
  Mock::given(method("POST"))
    .respond_with(ResponseTemplate::new(500))
    .mount(&server)
    .await;

  search(&server, "github", "plain").code(3).stderr(contains("registry"));
}

#[tokio::test(flavor = "multi_thread")]
async fn search_escapes_control_characters_from_the_registry() {
  let server = MockServer::start().await;
  mount(
    &server,
    "packages(",
    json!({ "packages": { "items": [
      { "orgSlug": "evil", "name": "hide\u{1b}[8m", "description": "Safe\u{1b}[1A\u{1b}[2K plugin",
        "latestVersion": null, "status": "VERIFIED", "middlewares": [] }
    ], "cursor": null } }),
  )
  .await;

  let assert = search(&server, "hide", "plain").success();
  let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
  assert!(!stdout.contains('\u{1b}'), "{stdout:?}");
  assert!(stdout.contains("Safe\\x1b[1A\\x1b[2K plugin"), "{stdout:?}");
  assert!(stdout.contains("evil/hide\\x1b[8m"), "{stdout:?}");
}

#[test]
fn search_refuses_plain_http_to_a_remote_registry() {
  Command::cargo_bin("moonlit")
    .unwrap()
    .args([
      "search",
      "github",
      "--registry",
      "http://registry.invalid",
      "--output",
      "plain",
    ])
    .assert()
    .code(2)
    .stderr(contains("http://registry.invalid"))
    .stderr(contains("https://"));
}

#[tokio::test(flavor = "multi_thread")]
async fn search_colors_badges_by_status_only_in_pretty_output() {
  let server = MockServer::start().await;
  mount(
    &server,
    "packages(",
    json!({ "packages": { "items": [
      { "orgSlug": "wolfware", "name": "good", "description": null,
        "latestVersion": null, "status": "VERIFIED", "middlewares": [] },
      { "orgSlug": "wolfware", "name": "bad", "description": null,
        "latestVersion": null, "status": "FAILED", "middlewares": [] },
      { "orgSlug": "wolfware", "name": "new", "description": null,
        "latestVersion": null, "status": "PENDING", "middlewares": [] }
    ], "cursor": null } }),
  )
  .await;
  let colored = |output: &str| {
    let assert = Command::cargo_bin("moonlit")
      .unwrap()
      .env("CLICOLOR_FORCE", "1")
      .env_remove("NO_COLOR")
      .args(["search", "x", "--registry", &server.uri(), "--output", output])
      .assert()
      .success();
    String::from_utf8(assert.get_output().stdout.clone()).unwrap()
  };

  let pretty = colored("pretty");
  assert!(pretty.contains("\u{1b}[32m✓ verified"), "{pretty:?}");
  assert!(pretty.contains("\u{1b}[31mfailed"), "{pretty:?}");
  assert!(pretty.contains("\u{1b}[2mpending"), "{pretty:?}");

  let plain = colored("plain");
  assert!(!plain.contains('\u{1b}'), "{plain:?}");
}
