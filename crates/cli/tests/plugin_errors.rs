use assert_cmd::Command;
use predicates::str::contains;
use std::path::{Path, PathBuf};

const EMPTY_COMPONENT: &[u8] = &[0x00, 0x61, 0x73, 0x6d, 0x0d, 0x00, 0x01, 0x00];
const GARBAGE: &[u8] = b"definitely not wasm";

fn fixture() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join("../engine/tests/fixtures/pdk_sample.wasm")
}

fn truncated_component(dir: &Path) -> PathBuf {
  let bytes = std::fs::read(fixture()).unwrap();
  write(dir, "truncated.wasm", &bytes[..bytes.len() / 2])
}

fn write(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
  let path = dir.join(name);
  std::fs::write(&path, bytes).unwrap();
  path
}

fn moonlit() -> Command {
  Command::cargo_bin("moonlit").unwrap()
}

fn plugin_crate(dir: &Path, lib_rs: Option<&str>) -> PathBuf {
  let root = dir.join("demo-plugin");
  std::fs::create_dir_all(root.join("src")).unwrap();
  std::fs::write(
    root.join("Cargo.toml"),
    "[package]\nname = \"demo-plugin\"\nversion = \"0.1.0\"\nedition = \"2021\"\nrepository = \"https://example.com/demo\"\nlicense = \"MIT\"\n\n[lib]\ncrate-type = [\"cdylib\"]\n\n[workspace]\n",
  )
  .unwrap();
  if let Some(src) = lib_rs {
    std::fs::write(root.join("src/lib.rs"), src).unwrap();
  }
  root
}

#[test]
fn inspect_reports_an_unreadable_path() {
  moonlit()
    .args(["plugin", "inspect", "no-such-plugin.wasm"])
    .assert()
    .code(2)
    .stderr(contains("cannot read 'no-such-plugin.wasm'"));
}

#[test]
fn inspect_reports_a_ref_that_does_not_resolve() {
  let dir = tempfile::tempdir().unwrap();
  let missing = dir.path().join("missing.wasm");
  moonlit()
    .env("MOONLIT_CACHE_DIR", dir.path())
    .args(["plugin", "inspect", &format!("file://{}", missing.display())])
    .assert()
    .code(3)
    .stderr(contains("error:"));
}

#[test]
fn inspect_rejects_bytes_that_are_not_wasm() {
  let dir = tempfile::tempdir().unwrap();
  let path = write(dir.path(), "garbage.wasm", GARBAGE);
  moonlit()
    .args(["plugin", "inspect"])
    .arg(&path)
    .assert()
    .code(2)
    .stderr(contains("not a valid wasm binary"));
}

#[test]
fn inspect_rejects_a_core_module() {
  let dir = tempfile::tempdir().unwrap();
  let path = write(dir.path(), "core.wasm", &[0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]);
  moonlit()
    .args(["plugin", "inspect"])
    .arg(&path)
    .assert()
    .code(2)
    .stderr(contains("core wasm module"));
}

#[test]
fn inspect_rejects_an_invalid_component() {
  let dir = tempfile::tempdir().unwrap();
  let path = truncated_component(dir.path());
  moonlit()
    .args(["plugin", "inspect"])
    .arg(&path)
    .assert()
    .code(2)
    .stderr(contains("invalid component"));
}

#[test]
fn inspect_reports_a_component_that_is_not_a_plugin() {
  let dir = tempfile::tempdir().unwrap();
  let path = write(dir.path(), "empty.wasm", EMPTY_COMPONENT);
  moonlit()
    .args(["plugin", "inspect"])
    .arg(&path)
    .assert()
    .code(3)
    .stderr(contains("failed to instantiate component"));
}

#[test]
fn inspect_pretty_prints_the_description() {
  moonlit()
    .args(["plugin", "inspect", "--output", "pretty"])
    .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../engine/tests/fixtures/test_plugin.wasm"))
    .assert()
    .success()
    .stdout(contains("Moonlit host test fixture"));
}

#[test]
fn inspect_pretty_prints_a_table() {
  moonlit()
    .args(["plugin", "inspect", "--output", "pretty"])
    .arg(fixture())
    .assert()
    .success()
    .stdout(contains("pdk-sample v"))
    .stdout(contains("Middleware"));
}

#[test]
fn publish_needs_a_crate_manifest_when_no_file_is_given() {
  let dir = tempfile::tempdir().unwrap();
  moonlit()
    .args(["plugin", "publish", "oci://reg.example.com/w/x:1", "--manifest-path"])
    .arg(dir.path())
    .assert()
    .code(2)
    .stderr(contains("cannot read"))
    .stderr(contains("pass --file"));
}

#[test]
fn publish_rejects_a_malformed_crate_manifest() {
  let dir = tempfile::tempdir().unwrap();
  std::fs::write(dir.path().join("Cargo.toml"), "not toml [").unwrap();
  moonlit()
    .args(["plugin", "publish", "oci://reg.example.com/w/x:1", "--manifest-path"])
    .arg(dir.path())
    .assert()
    .code(2)
    .stderr(contains("bad Cargo.toml"));
}

#[test]
fn publish_reports_when_cargo_cannot_describe_the_crate() {
  let dir = tempfile::tempdir().unwrap();
  let root = plugin_crate(dir.path(), None);
  moonlit()
    .args(["plugin", "publish", "oci://reg.example.com/w/x:1", "--manifest-path"])
    .arg(&root)
    .assert()
    .code(2)
    .stderr(contains("cargo metadata failed"));
}

#[test]
fn publish_points_at_the_release_build_when_it_is_missing() {
  let dir = tempfile::tempdir().unwrap();
  let root = plugin_crate(dir.path(), Some(""));
  moonlit()
    .args(["plugin", "publish", "oci://reg.example.com/w/x:1", "--manifest-path"])
    .arg(&root)
    .assert()
    .code(2)
    .stderr(contains("no built component"))
    .stderr(contains("demo_plugin.wasm"));
}

#[test]
fn publish_rejects_bytes_that_are_not_wasm() {
  let dir = tempfile::tempdir().unwrap();
  let path = write(dir.path(), "garbage.wasm", GARBAGE);
  moonlit()
    .args(["plugin", "publish", "reg.example.com/w/x:1", "--file"])
    .arg(&path)
    .assert()
    .code(2)
    .stderr(contains("not a valid wasm binary"));
}

#[test]
fn publish_rejects_an_invalid_component() {
  let dir = tempfile::tempdir().unwrap();
  let path = truncated_component(dir.path());
  moonlit()
    .args(["plugin", "publish", "reg.example.com/w/x:1", "--file"])
    .arg(&path)
    .assert()
    .code(2)
    .stderr(contains("invalid component"));
}

#[test]
fn publish_reports_a_component_that_is_not_a_plugin() {
  let dir = tempfile::tempdir().unwrap();
  let path = write(dir.path(), "empty.wasm", EMPTY_COMPONENT);
  moonlit()
    .args(["plugin", "publish", "reg.example.com/w/x:1", "--file"])
    .arg(&path)
    .assert()
    .code(3)
    .stderr(contains("failed to instantiate component"));
}

#[test]
fn publish_reports_a_registry_that_cannot_be_reached() {
  let dir = tempfile::tempdir().unwrap();
  let root = plugin_crate(dir.path(), Some(""));
  std::fs::write(
    root.join("Cargo.lock"),
    "[[package]]\nname = \"moonlit-pdk\"\nversion = \"0.4.1\"\n",
  )
  .unwrap();
  moonlit()
    .env("MOONLIT_HOME", dir.path())
    .args(["plugin", "publish", "oci://127.0.0.1:1/w/x:1", "--file"])
    .arg(fixture())
    .arg("--manifest-path")
    .arg(&root)
    .assert()
    .code(3)
    .stderr(contains("error:"));
}

#[test]
fn build_needs_a_crate_manifest() {
  let dir = tempfile::tempdir().unwrap();
  moonlit()
    .args(["plugin", "build", "--manifest-path"])
    .arg(dir.path())
    .assert()
    .code(2)
    .stderr(contains("cannot read"));
}

#[test]
fn build_rejects_a_malformed_crate_manifest() {
  let dir = tempfile::tempdir().unwrap();
  std::fs::write(dir.path().join("Cargo.toml"), "not toml [").unwrap();
  moonlit()
    .args(["plugin", "build", "--manifest-path"])
    .arg(dir.path())
    .assert()
    .code(2)
    .stderr(contains("bad Cargo.toml"));
}

#[test]
fn build_reports_a_failed_compile() {
  let dir = tempfile::tempdir().unwrap();
  let root = plugin_crate(dir.path(), Some("pub fn broken( {}\n"));
  moonlit()
    .args(["plugin", "build", "--manifest-path"])
    .arg(&root)
    .assert()
    .code(4)
    .stderr(contains("cargo build failed"));
}

#[test]
fn new_refuses_an_existing_directory_in_the_current_directory() {
  let dir = tempfile::tempdir().unwrap();
  std::fs::create_dir(dir.path().join("taken")).unwrap();
  moonlit()
    .current_dir(dir.path())
    .args(["plugin", "new", "taken"])
    .assert()
    .code(2)
    .stderr(contains("already exists"));
}
