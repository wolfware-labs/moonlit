use assert_cmd::Command;
use moonlit_engine::cache::{Cache, PluginMeta, SystemClock};
use predicates::str::contains;
use std::path::Path;

fn moonlit(cache: &Path) -> Command {
  let mut cmd = Command::cargo_bin("moonlit").unwrap();
  cmd.env("MOONLIT_CACHE_DIR", cache);
  cmd
}

fn seed(cache: &Path) {
  let meta = PluginMeta {
    source: "oci://reg.example.com/w/git:1.0.0".into(),
    digest: Some("sha256:0123456789abcdef0123456789abcdef".into()),
    layer_digest: None,
    size: 42,
    pulled_at: 1,
    middlewares: Some(vec!["build".into(), "test".into()]),
  };
  Cache::with_root_and_clock(cache.to_path_buf(), Box::new(SystemClock))
    .store_plugin("key-1", &meta, b"\0asm")
    .unwrap();
}

#[test]
fn plain_listing_shows_each_cached_plugin() {
  let cache = tempfile::tempdir().unwrap();
  seed(cache.path());
  moonlit(cache.path())
    .args(["cache", "ls", "--output", "plain"])
    .assert()
    .success()
    .stdout(contains(
      "oci://reg.example.com/w/git:1.0.0  sha256:0123456789abcdef0123456789abcdef  42 bytes  2 middlewares",
    ));
}

#[test]
fn json_listing_is_an_array_of_plugins() {
  let cache = tempfile::tempdir().unwrap();
  seed(cache.path());
  let out = moonlit(cache.path())
    .args(["cache", "ls", "--output", "json"])
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
  assert_eq!(v[0]["source"], "oci://reg.example.com/w/git:1.0.0");
  assert_eq!(v[0]["size"], 42);
  assert_eq!(v[0]["middlewares"], serde_json::json!(["build", "test"]));
}

#[test]
fn empty_json_listing_is_an_empty_array() {
  let cache = tempfile::tempdir().unwrap();
  let out = moonlit(cache.path())
    .args(["cache", "ls", "--output", "json"])
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
  assert_eq!(v, serde_json::json!([]));
}

#[test]
fn pretty_listing_is_a_table_with_a_short_digest() {
  let cache = tempfile::tempdir().unwrap();
  seed(cache.path());
  moonlit(cache.path())
    .args(["cache", "ls", "--output", "pretty"])
    .assert()
    .success()
    .stdout(contains("Reference"))
    .stdout(contains("sha256:0123456789ab "))
    .stdout(contains("42 B"));
}

#[test]
fn empty_pretty_listing_says_so() {
  let cache = tempfile::tempdir().unwrap();
  moonlit(cache.path())
    .args(["cache", "ls", "--output", "pretty"])
    .assert()
    .success()
    .stdout(contains("cache is empty"));
}

#[test]
fn clean_counts_and_removes_cached_plugins() {
  let cache = tempfile::tempdir().unwrap();
  seed(cache.path());
  moonlit(cache.path())
    .args(["cache", "clean"])
    .assert()
    .success()
    .stdout(contains("Removed 1 plugins"));
  assert!(!cache.path().join("plugins").exists());
}

#[test]
fn clean_reports_a_filesystem_error() {
  let cache = tempfile::tempdir().unwrap();
  std::fs::write(cache.path().join("plugins"), "not a directory").unwrap();
  moonlit(cache.path())
    .args(["cache", "clean"])
    .assert()
    .code(1)
    .stderr(contains("error:"));
}
