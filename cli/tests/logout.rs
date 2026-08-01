use assert_cmd::Command;
use predicates::str::contains;

/// Log in on the manual (CI) path, which stores a Basic `username`/`password` credential.
fn login_basic(home: &std::path::Path, host: &str) {
    Command::cargo_bin("moonlit")
        .unwrap()
        .env("HOME", home)
        .args(["login", host, "--username", "alice", "--token", "pat"])
        .assert()
        .success();
}

fn credentials(home: &std::path::Path) -> toml::Table {
    let text = std::fs::read_to_string(home.join(".config/moonlit/credentials.toml")).unwrap();
    text.parse().unwrap()
}

#[test]
fn logout_removes_a_basic_credential() {
    // A Basic credential has no `token` key, so gating logout on a Bearer lookup would leave the
    // plaintext password on disk with no CLI way to remove it.
    let home = tempfile::tempdir().unwrap();
    login_basic(home.path(), "ghcr.io");

    Command::cargo_bin("moonlit")
        .unwrap()
        .env("HOME", home.path())
        .args(["logout", "ghcr.io", "--local"])
        .assert()
        .success()
        .stdout(contains("Logged out of ghcr.io."));

    assert!(
        credentials(home.path())["registries"]
            .get("ghcr.io")
            .is_none()
    );
}

#[test]
fn logout_reports_not_logged_in_for_an_unknown_host() {
    let home = tempfile::tempdir().unwrap();
    login_basic(home.path(), "ghcr.io");

    Command::cargo_bin("moonlit")
        .unwrap()
        .env("HOME", home.path())
        .args(["logout", "other.example.com", "--local"])
        .assert()
        .success()
        .stdout(contains("Not logged in to other.example.com."));

    // The untouched host must survive.
    assert_eq!(
        credentials(home.path())["registries"]["ghcr.io"]["username"].as_str(),
        Some("alice")
    );
}

#[test]
fn logout_preserves_other_hosts() {
    let home = tempfile::tempdir().unwrap();
    login_basic(home.path(), "ghcr.io");
    login_basic(home.path(), "keep.example.com");

    Command::cargo_bin("moonlit")
        .unwrap()
        .env("HOME", home.path())
        .args(["logout", "ghcr.io", "--local"])
        .assert()
        .success();

    let doc = credentials(home.path());
    assert!(doc["registries"].get("ghcr.io").is_none());
    assert_eq!(
        doc["registries"]["keep.example.com"]["username"].as_str(),
        Some("alice")
    );
}
