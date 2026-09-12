use std::fs;
use std::path::Path;

use assert_cmd::Command;
use tempfile::tempdir;

fn fixture_wasm_url() -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../engine/tests/fixtures/test_plugin.wasm")
        .canonicalize()
        .expect("fixture wasm exists");
    format!("file://{}", p.display())
}

fn write_release(dir: &Path, middleware: &str) {
    let yaml = format!(
        "name: demo\nplugins:\n  - name: tp\n    url: {}\nstages:\n  build:\n    - name: s1\n      run: tp.{middleware}\n",
        fixture_wasm_url()
    );
    fs::write(dir.join("release.yml"), yaml).unwrap();
}

#[test]
fn successful_run_exits_zero() {
    let dir = tempdir().unwrap();
    write_release(dir.path(), "log-and-output");
    Command::cargo_bin("moonlit")
        .unwrap()
        .args(["run", "--output", "plain", "-w"])
        .arg(dir.path())
        .assert()
        .success();
}

#[test]
fn failing_step_exits_four() {
    let dir = tempdir().unwrap();
    write_release(dir.path(), "fail");
    Command::cargo_bin("moonlit")
        .unwrap()
        .args(["run", "--output", "plain", "-w"])
        .arg(dir.path())
        .assert()
        .code(4);
}

#[test]
fn unknown_middleware_exits_two() {
    let dir = tempdir().unwrap();
    write_release(dir.path(), "does-not-exist");
    Command::cargo_bin("moonlit")
        .unwrap()
        .args(["run", "--output", "plain", "-w"])
        .arg(dir.path())
        .assert()
        .code(2);
}

#[test]
fn validate_of_good_pipeline_exits_zero() {
    let dir = tempdir().unwrap();
    write_release(dir.path(), "log-and-output");
    Command::cargo_bin("moonlit")
        .unwrap()
        .args(["validate", "-w"])
        .arg(dir.path())
        .assert()
        .success();
}

#[test]
fn json_mode_emits_finished_event() {
    let dir = tempdir().unwrap();
    write_release(dir.path(), "log-and-output");
    let out = Command::cargo_bin("moonlit")
        .unwrap()
        .args(["run", "--output", "json", "-w"])
        .arg(dir.path())
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(
        stdout
            .lines()
            .any(|l| l.contains(r#""type":"pipeline_finished"#)),
        "expected a pipeline_finished json line; stdout:\n{stdout}"
    );
}

/// Diagnostics must name the file that was actually read. The source label was hardcoded to
/// `release.yml`, so a pipeline in `release.yaml` reported errors against a filename the user
/// does not have — and `release.yaml` only became a routine default recently.
#[test]
fn diagnostics_name_the_file_that_was_actually_read() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("release.yaml"), "pluigns:\n  - name: p\n").unwrap();
    let out = Command::cargo_bin("moonlit")
        .unwrap()
        .args(["run", "--output", "plain", "-w"])
        .arg(dir.path())
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        text.contains("release.yaml"),
        "diagnostic must name release.yaml, got:\n{text}"
    );
    assert!(
        !text.contains("release.yml:"),
        "diagnostic must not claim release.yml, got:\n{text}"
    );
}
