//! Black-box: `moonlit run` drives the moonlit plugin's `run-modules` via a file://
//! ref. Empty `modulePaths` hits the pinned guard before any subprocess —
//! deterministic and offline — asserting the CLI surfaces the frozen failure and
//! exits non-zero.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use predicates::str::contains;
use tempfile::tempdir;

fn wasm_url() -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../engine/tests/fixtures/moonlit.wasm")
        .canonicalize()
        .expect("moonlit.wasm fixture exists (regenerate via plugins/moonlit/README.md)");
    format!("file://{}", p.display())
}

#[test]
fn moonlit_run_empty_module_paths_fails() {
    let dir = tempdir().unwrap();
    let yaml = format!(
        "name: demo\n\
         plugins:\n\
         \x20 - name: moonlit\n\
         \x20   url: {url}\n\
         \x20   permissions:\n\
         \x20     exec: [moonlit]\n\
         stages:\n\
         \x20 release:\n\
         \x20   - name: modules\n\
         \x20     run: moonlit.run-modules\n\
         \x20     config:\n\
         \x20       modulePaths: []\n",
        url = wasm_url()
    );
    fs::write(dir.path().join("release.yml"), &yaml).unwrap();

    Command::cargo_bin("moonlit")
        .unwrap()
        .args(["run", "--output", "plain", "-w"])
        .arg(dir.path())
        .assert()
        .failure()
        .stderr(contains("No module paths provided for run-modules."));
}
