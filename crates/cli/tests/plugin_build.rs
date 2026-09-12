use assert_cmd::Command;

fn pdk_path() -> String {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../pdk").to_string()
}

fn wasm_target_installed() -> bool {
    let out = std::process::Command::new("rustc")
        .args(["--print", "sysroot"])
        .output();
    match out {
        Ok(o) if o.status.success() => {
            let root = String::from_utf8_lossy(&o.stdout);
            std::path::Path::new(root.trim())
                .join("lib/rustlib/wasm32-wasip2")
                .is_dir()
        }
        _ => false,
    }
}

#[test]
fn build_produces_and_validates_a_component() {
    if !wasm_target_installed() {
        eprintln!("SKIP: wasm32-wasip2 target not installed");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    // Scaffold, then build it — the end-to-end author loop.
    Command::cargo_bin("moonlit")
        .unwrap()
        .current_dir(dir.path())
        .args(["plugin", "new", "buildme", "--pdk-path", &pdk_path()])
        .assert()
        .success();

    let root = dir.path().join("buildme");
    Command::cargo_bin("moonlit")
        .unwrap()
        .current_dir(&root)
        .args(["plugin", "build", "--release"])
        .assert()
        .success();

    let artifact = root.join("target/wasm32-wasip2/release/buildme.wasm");
    assert!(artifact.is_file(), "component artifact should exist");

    // And it inspects cleanly (full author loop: new -> build -> inspect).
    Command::cargo_bin("moonlit")
        .unwrap()
        .args(["plugin", "inspect", artifact.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicates::str::contains("greet"));
}

#[test]
fn build_rejects_a_non_plugin_directory() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
    Command::cargo_bin("moonlit")
        .unwrap()
        .current_dir(dir.path())
        .args(["plugin", "build"])
        .assert()
        .code(2);
}
