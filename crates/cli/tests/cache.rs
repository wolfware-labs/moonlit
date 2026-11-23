#[path = "support/isolated.rs"]
mod isolated;

use predicates::str::contains;

#[test]
fn ls_on_empty_cache_reports_empty() {
  let cache = tempfile::tempdir().unwrap();
  isolated::moonlit()
    .env("MOONLIT_CACHE_DIR", cache.path())
    .args(["cache", "ls", "--output", "plain"])
    .assert()
    .success()
    .stdout(contains("cache is empty"));
}

#[test]
fn clean_reports_removed_counts() {
  let cache = tempfile::tempdir().unwrap();
  isolated::moonlit()
    .env("MOONLIT_CACHE_DIR", cache.path())
    .args(["cache", "clean"])
    .assert()
    .success()
    .stdout(contains("Removed 0 plugins"));
}
