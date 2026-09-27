use assert_cmd::Command;
use predicates::str::contains;
use std::path::Path;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn moonlit(home: &Path) -> Command {
  let mut cmd = Command::cargo_bin("moonlit").unwrap();
  cmd.env("MOONLIT_HOME", home);
  cmd
}

fn credentials(home: &Path) -> toml::Table {
  let text = std::fs::read_to_string(home.join(".config/moonlit/credentials.toml")).unwrap();
  text.parse().unwrap()
}

fn write_credentials(home: &Path, text: &str) {
  let path = home.join(".config/moonlit/credentials.toml");
  std::fs::create_dir_all(path.parent().unwrap()).unwrap();
  std::fs::write(path, text).unwrap();
}

#[test]
fn token_without_username_stores_a_bearer_credential() {
  let home = tempfile::tempdir().unwrap();
  moonlit(home.path())
    .args(["login", "ghcr.io", "--token", "tok"])
    .assert()
    .success()
    .stdout(contains("Logged in to ghcr.io."));
  let doc = credentials(home.path());
  assert_eq!(doc["registries"]["ghcr.io"]["token"].as_str(), Some("tok"));
  assert!(doc["registries"]["ghcr.io"].get("username").is_none());
}

#[test]
fn blank_username_falls_back_to_a_bearer_credential() {
  let home = tempfile::tempdir().unwrap();
  moonlit(home.path())
    .args(["login", "ghcr.io", "--username", "", "--token", "tok"])
    .assert()
    .success();
  assert_eq!(
    credentials(home.path())["registries"]["ghcr.io"]["token"].as_str(),
    Some("tok")
  );
}

#[test]
fn login_without_host_uses_the_default_registry() {
  let home = tempfile::tempdir().unwrap();
  moonlit(home.path())
    .args(["login", "--token", "tok"])
    .assert()
    .success()
    .stdout(contains("Logged in to registry.moonlit.rs."));
}

#[test]
fn empty_token_is_rejected() {
  let home = tempfile::tempdir().unwrap();
  moonlit(home.path())
    .args(["login", "ghcr.io", "--token", ""])
    .assert()
    .code(2)
    .stderr(contains("token must not be empty"));
}

#[test]
fn login_replaces_a_malformed_registries_entry() {
  let home = tempfile::tempdir().unwrap();
  write_credentials(home.path(), "registries = \"oops\"\n");
  moonlit(home.path())
    .args(["login", "ghcr.io", "--token", "tok"])
    .assert()
    .success();
  assert_eq!(
    credentials(home.path())["registries"]["ghcr.io"]["token"].as_str(),
    Some("tok")
  );
}

#[test]
fn login_fails_when_credentials_cannot_be_written() {
  let dir = tempfile::tempdir().unwrap();
  let home_is_a_file = dir.path().join("home");
  std::fs::write(&home_is_a_file, "").unwrap();
  moonlit(&home_is_a_file)
    .args(["login", "ghcr.io", "--token", "tok"])
    .assert()
    .code(1)
    .stderr(contains("failed to write credentials"));
}

#[test]
fn logout_without_credentials_reports_not_logged_in() {
  let home = tempfile::tempdir().unwrap();
  moonlit(home.path())
    .args(["logout", "ghcr.io"])
    .assert()
    .success()
    .stdout(contains("Not logged in to ghcr.io."));
}

#[test]
fn logout_without_a_registries_table_reports_not_logged_in() {
  let home = tempfile::tempdir().unwrap();
  write_credentials(home.path(), "other = 1\n");
  moonlit(home.path())
    .args(["logout", "--local"])
    .assert()
    .success()
    .stdout(contains("Not logged in to registry.moonlit.rs."));
}

async fn logout_with_revocation_status(status: u16) -> (tempfile::TempDir, assert_cmd::assert::Assert) {
  let server = MockServer::start().await;
  Mock::given(method("POST"))
    .and(path("/api/v1/device/logout"))
    .and(header("authorization", "Bearer tok"))
    .respond_with(ResponseTemplate::new(status))
    .expect(1)
    .mount(&server)
    .await;
  let host = server.uri().trim_start_matches("http://").to_string();
  let home = tempfile::tempdir().unwrap();
  write_credentials(home.path(), &format!("[registries.\"{host}\"]\ntoken = \"tok\"\n"));

  let assert = moonlit(home.path()).args(["logout", &host]).assert();
  (home, assert)
}

#[tokio::test(flavor = "multi_thread")]
async fn logout_revokes_a_bearer_token_on_the_server() {
  let (home, assert) = logout_with_revocation_status(200).await;
  assert
    .success()
    .stdout(contains("Logged out of"))
    .stderr(predicates::str::is_empty());
  assert!(credentials(home.path())["registries"].as_table().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn logout_warns_but_still_removes_when_revocation_fails() {
  let (home, assert) = logout_with_revocation_status(500).await;
  assert
    .success()
    .stderr(contains("could not revoke the token"))
    .stdout(contains("Logged out of"));
  assert!(credentials(home.path())["registries"].as_table().unwrap().is_empty());
}

#[test]
fn logout_fails_when_credentials_cannot_be_rewritten() {
  let home = tempfile::tempdir().unwrap();
  write_credentials(
    home.path(),
    "[registries.\"ghcr.io\"]
username = \"u\"
password = \"p\"
",
  );
  let path = home.path().join(".config/moonlit/credentials.toml");
  if cfg!(unix) {
    std::fs::create_dir(path.with_extension("toml.tmp")).unwrap();
  } else {
    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&path, perms).unwrap();
  }

  moonlit(home.path())
    .args(["logout", "ghcr.io", "--local"])
    .assert()
    .code(1)
    .stderr(contains("failed to update credentials"));
}
