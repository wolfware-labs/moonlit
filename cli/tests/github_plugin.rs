//! Black-box: run `github write-variables` through `moonlit run` via a file:// ref.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use tempfile::tempdir;

fn github_wasm_url() -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../engine/tests/fixtures/github.wasm")
        .canonicalize()
        .expect("github.wasm fixture exists (regenerate via plugins/github/README.md)");
    format!("file://{}", p.display())
}

#[test]
fn moonlit_run_drives_github_write_variables() {
    let dir = tempdir().unwrap();
    let out_file = dir.path().join("gh_output");
    fs::write(&out_file, "").unwrap();

    let yaml = format!(
        "name: demo\n\
         plugins:\n\
         \x20 - name: github\n\
         \x20   url: {url}\n\
         \x20   config:\n\
         \x20     token: dummy\n\
         \x20   permissions:\n\
         \x20     exec: [sh]\n\
         \x20     env: [\"*\"]\n\
         \x20     filesystem: read-write\n\
         stages:\n\
         \x20 release:\n\
         \x20   - name: vars\n\
         \x20     run: github.write-variables\n\
         \x20     config:\n\
         \x20       output:\n\
         \x20         version: 1.2.3\n",
        url = github_wasm_url()
    );
    fs::write(dir.path().join("release.yml"), &yaml).unwrap();

    Command::cargo_bin("moonlit")
        .unwrap()
        .args(["run", "--output", "plain", "-w"])
        .arg(dir.path())
        .env("GITHUB_OUTPUT", &out_file)
        .assert()
        .success();

    let written = fs::read_to_string(&out_file).unwrap();
    assert!(written.contains("version=1.2.3\n"), "got: {written}");
}
