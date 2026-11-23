#[path = "support/isolated.rs"]
mod isolated;

use predicates::str::contains;

#[test]
fn manual_path_without_token_errors_non_interactive() {
  // `--username` selects the manual (CI) path; without `--token` on a non-TTY it must error
  // rather than prompt. A bare `login <host>` with no flags now runs the device flow instead.
  isolated::moonlit()
    .args(["login", "ghcr.io", "--username", "alice"])
    .assert()
    .code(2)
    .stderr(contains("requires --token"));
}

#[test]
fn stores_basic_credentials_under_home() {
  let home = tempfile::tempdir().unwrap();
  isolated::moonlit()
    .env("MOONLIT_HOME", home.path())
    .args(["login", "ghcr.io", "--username", "alice", "--token", "pat"])
    .assert()
    .success()
    .stdout(contains("Logged in to ghcr.io."));

  let text = std::fs::read_to_string(home.path().join(".config/moonlit/credentials.toml")).unwrap();
  let doc: toml::Table = text.parse().unwrap();
  assert_eq!(doc["registries"]["ghcr.io"]["username"].as_str(), Some("alice"));
  assert_eq!(doc["registries"]["ghcr.io"]["password"].as_str(), Some("pat"));
}
