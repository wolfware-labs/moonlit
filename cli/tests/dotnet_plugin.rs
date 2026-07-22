//! Black-box: `moonlit run` drives the dotnet plugin's `build` middleware via a file://
//! ref. A missing project fails before any `dotnet` spawn, so the run is deterministic
//! and needs no .NET SDK — asserting the CLI surfaces the frozen failure and exits non-zero.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use predicates::str::contains;
use tempfile::tempdir;

fn wasm_url() -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../engine/tests/fixtures/dotnet.wasm")
        .canonicalize()
        .expect("dotnet.wasm fixture exists (regenerate via plugins/dotnet/README.md)");
    format!("file://{}", p.display())
}

#[test]
fn moonlit_run_dotnet_build_missing_project_fails() {
    let dir = tempdir().unwrap();
    let yaml = format!(
        "name: demo\n\
         plugins:\n\
         \x20 - name: dotnet\n\
         \x20   url: {url}\n\
         \x20   permissions:\n\
         \x20     exec: [dotnet]\n\
         \x20     filesystem: read-write\n\
         stages:\n\
         \x20 release:\n\
         \x20   - name: build\n\
         \x20     run: dotnet.build\n\
         \x20     config:\n\
         \x20       project: missing.csproj\n\
         \x20       version: 1.0.0\n",
        url = wasm_url()
    );
    fs::write(dir.path().join("release.yml"), &yaml).unwrap();

    Command::cargo_bin("moonlit")
        .unwrap()
        .args(["run", "--output", "plain", "-w"])
        .arg(dir.path())
        .assert()
        .failure()
        .stderr(contains("Project file not found at path:"));
}
