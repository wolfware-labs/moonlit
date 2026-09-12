use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;

#[test]
fn missing_file_errors_with_hint() {
    Command::cargo_bin("moonlit")
        .unwrap()
        .args([
            "plugin",
            "publish",
            "oci://reg.example.com/w/x:1",
            "--file",
            "/nonexistent/plugin.wasm",
        ])
        .assert()
        .code(2)
        .stderr(contains("no built component").and(contains("moonlit plugin build")));
}

#[test]
fn core_module_is_rejected() {
    // Smallest valid core module (magic + version 1): not a component.
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("core.wasm");
    std::fs::write(&path, [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]).unwrap();
    Command::cargo_bin("moonlit")
        .unwrap()
        .args(["plugin", "publish", "oci://reg.example.com/w/x:1", "--file"])
        .arg(&path)
        .assert()
        .code(2)
        .stderr(contains("core wasm module"));
}
