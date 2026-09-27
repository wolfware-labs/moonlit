use assert_cmd::Command;
use predicates::str::contains;
use std::path::Path;

fn fixture_wasm_url() -> String {
  let p = Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../engine/tests/fixtures/test_plugin.wasm")
    .canonicalize()
    .expect("fixture wasm exists");
  format!("file://{}", p.display())
}

fn unwrapped_stderr(output: &std::process::Output) -> String {
  String::from_utf8_lossy(&output.stderr)
    .split_whitespace()
    .filter(|word| *word != "│")
    .collect::<Vec<_>>()
    .join(" ")
}

fn write_release(path: &Path) {
  let yaml = format!(
    "name: demo\nplugins:\n  - name: tp\n    url: {}\nstages:\n  build:\n    - name: s1\n      run: tp.log-and-output\n  other:\n    - name: s2\n      run: tp.log-and-output\n",
    fixture_wasm_url()
  );
  std::fs::write(path, yaml).unwrap();
}

fn moonlit_in(dir: &Path) -> Command {
  let mut cmd = Command::cargo_bin("moonlit").unwrap();
  cmd.current_dir(dir);
  cmd
}

#[test]
fn run_finds_release_yaml_in_the_current_directory() {
  let dir = tempfile::tempdir().unwrap();
  write_release(&dir.path().join("release.yaml"));
  moonlit_in(dir.path())
    .args(["run", "--output", "plain", "-v", "-s", "build", "-a", "who=world"])
    .assert()
    .success()
    .stderr(contains("stages: build"))
    .stderr(contains("s1"));
}

#[test]
fn validate_finds_release_yml_in_the_current_directory() {
  let dir = tempfile::tempdir().unwrap();
  write_release(&dir.path().join("release.yml"));
  moonlit_in(dir.path())
    .args(["validate", "--output", "plain"])
    .assert()
    .success()
    .stderr(contains("Configuration valid"));
}

#[test]
fn pretty_run_succeeds_without_a_terminal() {
  let dir = tempfile::tempdir().unwrap();
  write_release(&dir.path().join("release.yml"));
  moonlit_in(dir.path()).args(["run", "--output", "pretty"]).assert().success();
}

#[test]
fn run_without_a_pipeline_file_names_the_defaults() {
  let dir = tempfile::tempdir().unwrap();
  let assert = moonlit_in(dir.path()).args(["run", "--output", "plain"]).assert().code(2);
  let stderr = unwrapped_stderr(assert.get_output());
  assert!(stderr.contains("No pipeline file found"), "{stderr}");
  assert!(stderr.contains("(looked for release.yml, release.yaml)"), "{stderr}");
}

#[test]
fn run_with_a_missing_file_reports_it() {
  let dir = tempfile::tempdir().unwrap();
  let assert = moonlit_in(dir.path())
    .args(["run", "--output", "plain", "-f", "missing.yml"])
    .assert()
    .code(2);
  let stderr = unwrapped_stderr(assert.get_output());
  assert!(stderr.contains("missing.yml' does not exist."), "{stderr}");
}

#[test]
fn json_mode_reports_errors_as_a_json_line() {
  let dir = tempfile::tempdir().unwrap();
  let out = moonlit_in(dir.path())
    .args(["run", "--output", "json", "-f", "missing.yml"])
    .assert()
    .code(2)
    .get_output()
    .stdout
    .clone();
  let line: serde_json::Value = serde_json::from_slice(&out).unwrap();
  assert_eq!(line["type"], "error");
  assert_eq!(line["exit_code"], 2);
}

#[test]
fn json_mode_reports_pipeline_errors_as_a_json_line() {
  let dir = tempfile::tempdir().unwrap();
  std::fs::write(dir.path().join("release.yml"), "pluigns: []\n").unwrap();
  let out = moonlit_in(dir.path())
    .args(["run", "--output", "json"])
    .assert()
    .code(2)
    .get_output()
    .stdout
    .clone();
  let text = String::from_utf8(out).unwrap();
  assert!(text.lines().any(|l| l.contains(r#""type":"error""#)), "{text}");
}

#[test]
fn invalid_arg_is_rejected_by_the_parser() {
  let dir = tempfile::tempdir().unwrap();
  moonlit_in(dir.path())
    .args(["run", "-a", "novalue"])
    .assert()
    .code(2)
    .stderr(contains("expected key=value"));
}

#[test]
fn unreadable_pipeline_file_is_reported() {
  let dir = tempfile::tempdir().unwrap();
  std::fs::write(dir.path().join("release.yml"), [0xff, 0xfe, 0xfd]).unwrap();
  moonlit_in(dir.path())
    .args(["run", "--output", "plain"])
    .assert()
    .code(2)
    .stderr(contains("Error while reading"));
}
