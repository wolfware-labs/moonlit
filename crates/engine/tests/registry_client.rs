use moonlit_engine::registry::{RegistryClient, RegistryError, VerificationStatus, VersionRule};
use serde_json::{Value, json};
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn client(server: &MockServer) -> RegistryClient {
  RegistryClient::with_base_url(&format!("{}/graphql", server.uri())).expect("loopback")
}

async fn mount_data(server: &MockServer, marker: &str, data: Value) {
  Mock::given(method("POST"))
    .and(path("/graphql"))
    .and(body_string_contains(marker))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": data })))
    .mount(server)
    .await;
}

async fn mount_package(server: &MockServer, versions: Value) {
  mount_data(server, "package(org", json!({ "package": { "versions": versions } })).await;
}

async fn mount_version(server: &MockServer, version: &str, world: &str) {
  Mock::given(method("POST"))
    .and(path("/graphql"))
    .and(body_string_contains("packageVersion"))
    .and(body_string_contains(format!("\"version\":\"{version}\"")))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
      "data": { "packageVersion": {
        "version": version,
        "worldVersion": world,
        "status": "VERIFIED",
        "middlewares": [{ "name": "run", "description": "Runs it" }]
      } }
    })))
    .mount(server)
    .await;
}

fn summary(version: &str, yanked: bool) -> Value {
  json!({ "version": version, "yanked": yanked, "status": "VERIFIED" })
}

#[tokio::test]
async fn search_sends_query_and_limit() {
  let server = MockServer::start().await;
  Mock::given(method("POST"))
    .and(path("/graphql"))
    .and(body_string_contains("\"query\":\"github\""))
    .and(body_string_contains("\"limit\":10"))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
      "data": { "packages": { "items": [
        { "orgSlug": "acme", "name": "github", "description": "GitHub tools",
          "latestVersion": "1.2.0", "status": "VERIFIED", "middlewares": ["issues", "prs"] },
        { "orgSlug": "acme", "name": "github-actions", "description": null,
          "latestVersion": null, "status": "PENDING", "middlewares": [] }
      ], "cursor": null } }
    })))
    .expect(1)
    .mount(&server)
    .await;

  let hits = client(&server).search("github", 10).await.unwrap();

  assert_eq!(hits.len(), 2);
  assert_eq!(hits[0].org, "acme");
  assert_eq!(hits[0].name, "github");
  assert_eq!(hits[0].description.as_deref(), Some("GitHub tools"));
  assert_eq!(hits[0].latest_version.as_deref(), Some("1.2.0"));
  assert_eq!(hits[0].status, VerificationStatus::Verified);
  assert_eq!(hits[0].middlewares, vec!["issues", "prs"]);
  assert_eq!(hits[1].status, VerificationStatus::Pending);
  assert_eq!(hits[1].description, None);
}

#[tokio::test]
async fn newest_compatible_skips_yanked_and_wrong_world() {
  let server = MockServer::start().await;
  mount_package(
    &server,
    json!([summary("1.9.0", false), summary("2.1.0", true), summary("2.0.0", false)]),
  )
  .await;
  mount_version(&server, "2.0.0", "0.4.0").await;
  mount_version(&server, "1.9.0", "0.4.0").await;
  mount_version(&server, "2.1.0", "0.4.0").await;

  let rule = VersionRule {
    current: None,
    cross_major: true,
    pre: false,
  };
  let found = client(&server)
    .newest_compatible("acme", "tool", "0.4.0", rule)
    .await
    .unwrap()
    .unwrap();

  assert_eq!(found.version, "2.0.0");
  assert_eq!(found.middlewares[0].name, "run");
}

#[tokio::test]
async fn newest_compatible_skips_versions_built_for_another_world() {
  let server = MockServer::start().await;
  mount_package(&server, json!([summary("2.0.0", false), summary("1.9.0", false)])).await;
  mount_version(&server, "2.0.0", "0.3.0").await;
  mount_version(&server, "1.9.0", "0.4.0").await;

  let rule = VersionRule {
    current: None,
    cross_major: true,
    pre: false,
  };
  let found = client(&server)
    .newest_compatible("acme", "tool", "0.4.0", rule)
    .await
    .unwrap()
    .unwrap();

  assert_eq!(found.version, "1.9.0");
}

#[tokio::test]
async fn newest_compatible_skips_details_for_a_version_it_did_not_ask_for() {
  let server = MockServer::start().await;
  mount_package(&server, json!([summary("1.3.0", false), summary("1.2.5", false)])).await;
  Mock::given(method("POST"))
    .and(path("/graphql"))
    .and(body_string_contains("packageVersion"))
    .and(body_string_contains("\"version\":\"1.3.0\""))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
      "data": { "packageVersion": {
        "version": "2.0.0-rc.1", "worldVersion": "0.4.0", "status": "VERIFIED", "middlewares": []
      } }
    })))
    .mount(&server)
    .await;
  mount_version(&server, "1.2.5", "0.4.0").await;

  let rule = VersionRule {
    current: Some(semver::Version::parse("1.2.0").unwrap()),
    cross_major: false,
    pre: false,
  };
  let found = client(&server)
    .newest_compatible("acme", "tool", "0.4.0", rule)
    .await
    .unwrap()
    .unwrap();

  assert_eq!(found.version, "1.2.5");
}

#[tokio::test]
async fn newest_compatible_stays_in_major() {
  let server = MockServer::start().await;
  mount_package(
    &server,
    json!([summary("2.0.0", false), summary("1.3.0", false), summary("1.2.0", false)]),
  )
  .await;
  mount_version(&server, "2.0.0", "0.4.0").await;
  mount_version(&server, "1.3.0", "0.4.0").await;

  let current = semver::Version::parse("1.2.0").unwrap();
  let same = VersionRule {
    current: Some(current.clone()),
    cross_major: false,
    pre: false,
  };
  let found = client(&server)
    .newest_compatible("acme", "tool", "0.4.0", same)
    .await
    .unwrap()
    .unwrap();
  assert_eq!(found.version, "1.3.0");

  let cross = VersionRule {
    current: Some(current),
    cross_major: true,
    pre: false,
  };
  let found = client(&server)
    .newest_compatible("acme", "tool", "0.4.0", cross)
    .await
    .unwrap()
    .unwrap();
  assert_eq!(found.version, "2.0.0");
}

#[tokio::test]
async fn newest_compatible_ignores_prereleases_unless_asked() {
  let server = MockServer::start().await;
  mount_package(&server, json!([summary("2.0.0-beta.1", false), summary("1.0.0", false)])).await;
  mount_version(&server, "2.0.0-beta.1", "0.4.0").await;
  mount_version(&server, "1.0.0", "0.4.0").await;

  let stable = VersionRule {
    current: None,
    cross_major: true,
    pre: false,
  };
  let found = client(&server)
    .newest_compatible("acme", "tool", "0.4.0", stable)
    .await
    .unwrap()
    .unwrap();
  assert_eq!(found.version, "1.0.0");

  let from_stable = VersionRule {
    current: Some(semver::Version::parse("1.0.0").unwrap()),
    cross_major: true,
    pre: false,
  };
  let found = client(&server)
    .newest_compatible("acme", "tool", "0.4.0", from_stable)
    .await
    .unwrap()
    .unwrap();
  assert_eq!(found.version, "1.0.0");

  let pre = VersionRule {
    current: None,
    cross_major: true,
    pre: true,
  };
  let found = client(&server)
    .newest_compatible("acme", "tool", "0.4.0", pre)
    .await
    .unwrap()
    .unwrap();
  assert_eq!(found.version, "2.0.0-beta.1");
}

#[tokio::test]
async fn package_version_is_none_when_absent() {
  let server = MockServer::start().await;
  mount_data(&server, "packageVersion", json!({ "packageVersion": null })).await;

  let found = client(&server).package_version("acme", "tool", "9.9.9").await.unwrap();

  assert!(found.is_none());
}

#[tokio::test]
async fn versions_lists_summaries() {
  let server = MockServer::start().await;
  mount_package(&server, json!([summary("1.0.0", true)])).await;

  let versions = client(&server).versions("acme", "tool").await.unwrap();

  assert_eq!(versions.len(), 1);
  assert!(versions[0].yanked);
  assert_eq!(versions[0].status, VerificationStatus::Verified);
}

#[tokio::test]
async fn graphql_errors_surface_message() {
  let server = MockServer::start().await;
  Mock::given(method("POST"))
    .and(path("/graphql"))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
      "errors": [{ "message": "boom" }, { "message": "second" }]
    })))
    .mount(&server)
    .await;

  let err = client(&server).search("x", 5).await.unwrap_err();

  assert!(matches!(&err, RegistryError::GraphQl(m) if m == "boom"), "{err:?}");
}

#[tokio::test]
async fn http_failures_are_reported() {
  let server = MockServer::start().await;
  Mock::given(method("POST"))
    .respond_with(ResponseTemplate::new(500))
    .mount(&server)
    .await;

  let err = client(&server).search("x", 5).await.unwrap_err();

  assert!(matches!(err, RegistryError::Http(_)), "{err:?}");
}

#[tokio::test]
async fn a_host_without_a_graphql_endpoint_is_not_a_registry() {
  for status in [404, 405] {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
      .respond_with(ResponseTemplate::new(status))
      .mount(&server)
      .await;

    let err = client(&server).versions("acme", "x").await.unwrap_err();

    assert!(matches!(err, RegistryError::NotARegistry(_)), "{status}: {err:?}");
  }
}

#[tokio::test]
async fn a_host_that_refuses_anonymous_graphql_is_denied_with_its_status() {
  for status in [401, 403] {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
      .respond_with(ResponseTemplate::new(status))
      .mount(&server)
      .await;

    let err = client(&server).versions("acme", "x").await.unwrap_err();

    assert!(
      matches!(&err, RegistryError::Denied { status: s, .. } if *s == status),
      "{status}: {err:?}"
    );
  }
}

#[tokio::test]
async fn a_response_that_is_not_a_graphql_envelope_is_not_a_registry() {
  let server = MockServer::start().await;
  Mock::given(method("POST"))
    .respond_with(ResponseTemplate::new(200).set_body_string("<html>hello</html>"))
    .mount(&server)
    .await;

  let err = client(&server).versions("acme", "x").await.unwrap_err();

  assert!(matches!(err, RegistryError::NotARegistry(_)), "{err:?}");
}

#[tokio::test]
async fn a_registry_that_does_not_answer_in_time_is_unreachable() {
  let server = MockServer::start().await;
  Mock::given(method("POST"))
    .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(5)))
    .mount(&server)
    .await;
  let client = client(&server).with_timeout(std::time::Duration::from_millis(200));

  let started = std::time::Instant::now();
  let err = client.search("x", 5).await.unwrap_err();

  assert!(matches!(err, RegistryError::Unreachable(_)), "{err:?}");
  assert!(
    started.elapsed() < std::time::Duration::from_secs(2),
    "{:?}",
    started.elapsed()
  );
}

#[test]
fn for_host_builds_the_graphql_url() {
  let url = |input: &str| RegistryClient::for_host(input).expect("allowed").base_url().to_string();
  assert_eq!(url("registry.moonlit.rs"), "https://registry.moonlit.rs/graphql");
  assert_eq!(url("https://registry.moonlit.rs"), "https://registry.moonlit.rs/graphql");
  assert_eq!(
    url("https://registry.moonlit.rs/graphql"),
    "https://registry.moonlit.rs/graphql"
  );
  assert_eq!(url("https://registry.moonlit.rs/"), "https://registry.moonlit.rs/graphql");
  assert_eq!(
    url("https://registry.moonlit.rs/graphql/"),
    "https://registry.moonlit.rs/graphql"
  );
  assert_eq!(url("http://127.0.0.1:9"), "http://127.0.0.1:9/graphql");
  assert_eq!(url("127.0.0.1:9"), "http://127.0.0.1:9/graphql");
  assert_eq!(url("localhost:9"), "http://localhost:9/graphql");
  assert_eq!(url("127.evil.com"), "https://127.evil.com/graphql");
}

#[test]
fn plain_http_is_refused_for_a_remote_registry() {
  for url in [
    "http://example.com",
    "http://example.com:8080/graphql",
    "http://127.evil.com",
    "http://localhost@example.com/graphql",
    "http://localhost:abc/graphql",
    "HTTP://example.com/graphql",
    "Http://example.com",
  ] {
    let err = RegistryClient::for_host(url).expect_err(url);
    assert!(matches!(err, RegistryError::Insecure(_)), "{err:?}");
    let err = RegistryClient::with_base_url(url).expect_err(url);
    assert!(matches!(err, RegistryError::Insecure(_)), "{err:?}");
  }
  for url in [
    "http://localhost:9",
    "http://[::1]:9/graphql",
    "http://127.0.0.2/graphql",
    "https://example.com",
  ] {
    assert!(RegistryClient::for_host(url).is_ok(), "{url}");
    assert!(RegistryClient::with_base_url(url).is_ok(), "{url}");
  }
}

#[test]
fn with_base_url_is_verbatim() {
  assert_eq!(
    RegistryClient::with_base_url("http://127.0.0.1:9/x")
      .expect("loopback")
      .base_url(),
    "http://127.0.0.1:9/x"
  );
}

async fn lookups(server: &MockServer) -> usize {
  server
    .received_requests()
    .await
    .unwrap()
    .iter()
    .filter(|r| String::from_utf8_lossy(&r.body).contains("query packageVersion("))
    .count()
}

fn any_rule() -> VersionRule {
  VersionRule {
    current: None,
    cross_major: true,
    pre: false,
  }
}

#[tokio::test]
async fn newest_compatible_finds_a_compatible_version_behind_incompatible_newer_ones() {
  let server = MockServer::start().await;
  let all: Vec<String> = (0..14).map(|i| format!("1.0.{i}")).collect();
  mount_package(&server, json!(all.iter().map(|v| summary(v, false)).collect::<Vec<_>>())).await;
  for version in &all {
    let world = if version == "1.0.10" { "0.4.0" } else { "0.3.0" };
    mount_version(&server, version, world).await;
  }

  let found = client(&server)
    .newest_compatible("acme", "tool", "0.4.0", any_rule())
    .await
    .unwrap();

  assert_eq!(found.unwrap().version, "1.0.10");
  assert!(lookups(&server).await <= 10);
}

#[tokio::test]
async fn newest_compatible_asks_about_every_candidate_when_none_matches() {
  let server = MockServer::start().await;
  let all: Vec<String> = (0..25).map(|i| format!("1.0.{i}")).collect();
  mount_package(&server, json!(all.iter().map(|v| summary(v, false)).collect::<Vec<_>>())).await;
  for version in &all {
    mount_version(&server, version, "0.3.0").await;
  }

  let found = client(&server)
    .newest_compatible("acme", "tool", "0.4.0", any_rule())
    .await
    .unwrap();

  assert!(found.is_none());
  assert_eq!(lookups(&server).await, 25);
}

#[tokio::test]
async fn newest_compatible_looks_past_the_first_ten_and_stops_at_the_batch_that_matches() {
  let server = MockServer::start().await;
  let all: Vec<String> = (0..25).map(|i| format!("1.0.{i}")).collect();
  mount_package(&server, json!(all.iter().map(|v| summary(v, false)).collect::<Vec<_>>())).await;
  for version in &all {
    let world = if version == "1.0.12" || version == "1.0.3" {
      "0.4.0"
    } else {
      "0.3.0"
    };
    mount_version(&server, version, world).await;
  }

  let found = client(&server)
    .newest_compatible("acme", "tool", "0.4.0", any_rule())
    .await
    .unwrap();

  assert_eq!(found.unwrap().version, "1.0.12");
  assert_eq!(lookups(&server).await, 20);
}

#[tokio::test]
async fn newest_compatible_prefers_the_newest_even_when_it_answers_last() {
  let server = MockServer::start().await;
  let all: Vec<String> = (0..14).map(|i| format!("1.0.{i}")).collect();
  mount_package(&server, json!(all.iter().map(|v| summary(v, false)).collect::<Vec<_>>())).await;
  for version in &all {
    let world = if version == "1.0.13" || version == "1.0.12" {
      "0.4.0"
    } else {
      "0.3.0"
    };
    if version == "1.0.13" {
      Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(body_string_contains("packageVersion"))
        .and(body_string_contains("\"version\":\"1.0.13\""))
        .respond_with(
          ResponseTemplate::new(200)
            .set_delay(std::time::Duration::from_millis(300))
            .set_body_json(json!({ "data": { "packageVersion": {
              "version": version, "worldVersion": world, "status": "VERIFIED", "middlewares": []
            } } })),
        )
        .mount(&server)
        .await;
    } else {
      mount_version(&server, version, world).await;
    }
  }

  let found = client(&server)
    .newest_compatible("acme", "tool", "0.4.0", any_rule())
    .await
    .unwrap();

  assert_eq!(found.unwrap().version, "1.0.13");
}

#[tokio::test]
async fn a_redirect_to_plain_http_on_a_remote_host_is_refused() {
  let server = MockServer::start().await;
  Mock::given(method("POST"))
    .respond_with(ResponseTemplate::new(307).insert_header("location", "http://192.0.2.1/graphql"))
    .mount(&server)
    .await;

  let err = client(&server).versions("acme", "x").await.unwrap_err();

  assert!(
    matches!(&err, RegistryError::Insecure(url) if url.contains("192.0.2.1")),
    "{err:?}"
  );
}

#[tokio::test]
async fn a_redirect_to_another_loopback_url_is_followed() {
  let server = MockServer::start().await;
  Mock::given(method("POST"))
    .and(path("/graphql"))
    .respond_with(ResponseTemplate::new(307).insert_header("location", format!("{}/moved", server.uri())))
    .mount(&server)
    .await;
  Mock::given(method("POST"))
    .and(path("/moved"))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": { "package": { "versions": [] } } })))
    .mount(&server)
    .await;

  let versions = client(&server).versions("acme", "x").await.expect("followed");

  assert!(versions.is_empty(), "{versions:?}");
}

#[tokio::test]
async fn a_json_body_without_data_or_errors_is_not_a_registry() {
  let server = MockServer::start().await;
  Mock::given(method("POST"))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
    .mount(&server)
    .await;

  let err = client(&server).versions("acme", "x").await.unwrap_err();

  assert!(matches!(err, RegistryError::NotARegistry(_)), "{err:?}");
}

#[tokio::test]
async fn a_redirect_loop_gives_up() {
  let server = MockServer::start().await;
  Mock::given(method("POST"))
    .and(path("/graphql"))
    .respond_with(ResponseTemplate::new(307).insert_header("location", format!("{}/graphql", server.uri())))
    .mount(&server)
    .await;

  let err = client(&server).versions("acme", "x").await.unwrap_err();

  assert!(matches!(err, RegistryError::Unreachable(_)), "{err:?}");
}
