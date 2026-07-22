//! Black-box: run `gitlab write-variables` through `moonlit run` via a file:// ref.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use tempfile::tempdir;

fn gitlab_wasm_url() -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../engine/tests/fixtures/gitlab.wasm")
        .canonicalize()
        .expect("gitlab.wasm fixture exists (regenerate via plugins/gitlab/README.md)");
    format!("file://{}", p.display())
}

#[test]
fn moonlit_run_drives_gitlab_write_variables() {
    let dir = tempdir().unwrap();
    let yaml = format!(
        "name: demo\n\
         plugins:\n\
         \x20 - name: gitlab\n\
         \x20   url: {url}\n\
         \x20   config:\n\
         \x20     token: dummy\n\
         \x20   permissions:\n\
         \x20     filesystem: read-write\n\
         stages:\n\
         \x20 release:\n\
         \x20   - name: vars\n\
         \x20     run: gitlab.write-variables\n\
         \x20     config:\n\
         \x20       output:\n\
         \x20         version: 1.2.3\n",
        url = gitlab_wasm_url()
    );
    fs::write(dir.path().join("release.yml"), &yaml).unwrap();

    Command::cargo_bin("moonlit")
        .unwrap()
        .args(["run", "--output", "plain", "-w"])
        .arg(dir.path())
        .assert()
        .success();

    // write-variables wrote moonlit.env into the working dir (no env var, no exec needed).
    let written = fs::read_to_string(dir.path().join("moonlit.env")).unwrap();
    assert!(written.contains("version=1.2.3\n"), "got: {written}");
}
