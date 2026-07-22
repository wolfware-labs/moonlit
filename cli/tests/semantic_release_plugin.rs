//! Black-box: run the semantic-release trio through `moonlit run` via a file:// ref,
//! asserting `moonlit run` drives the analyze -> calculate-version trio end-to-end
//! and exits successfully.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use tempfile::tempdir;

fn wasm_url() -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../engine/tests/fixtures/semantic-release.wasm")
        .canonicalize()
        .expect("semantic-release.wasm fixture exists (regenerate via plugins/semantic-release/README.md)");
    format!("file://{}", p.display())
}

#[test]
fn moonlit_run_drives_semantic_release_trio() {
    let dir = tempdir().unwrap();
    let yaml = format!(
        "name: demo\n\
         plugins:\n\
         \x20 - name: semantic-release\n\
         \x20   url: {url}\n\
         stages:\n\
         \x20 release:\n\
         \x20   - name: analyze\n\
         \x20     run: semantic-release.analyze\n\
         \x20     config:\n\
         \x20       commits:\n\
         \x20         - sha: abc1234def\n\
         \x20           date: 2026-01-01T00:00:00Z\n\
         \x20           message: 'feat: add thing'\n\
         \x20   - name: version\n\
         \x20     run: semantic-release.calculate-version\n\
         \x20     config:\n\
         \x20       baseVersion: 1.2.3\n",
        url = wasm_url()
    );
    fs::write(dir.path().join("release.yml"), &yaml).unwrap();

    Command::cargo_bin("moonlit")
        .unwrap()
        .args(["run", "--output", "plain", "-w"])
        .arg(dir.path())
        .assert()
        .success();
}
