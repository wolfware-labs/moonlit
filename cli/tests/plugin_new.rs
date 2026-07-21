use assert_cmd::Command;

/// Absolute path to the in-repo SDK, so the scaffold builds without a published crate.
fn sdk_path() -> String {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../sdk").to_string()
}

#[test]
fn new_scaffolds_a_buildable_plugin() {
    let dir = tempfile::tempdir().unwrap();
    // Non-TTY (assert_cmd pipes stdin): must not prompt, must apply defaults.
    Command::cargo_bin("moonlit")
        .unwrap()
        .current_dir(dir.path())
        .args([
            "plugin",
            "new",
            "my-plugin",
            "--sdk-path",
            &sdk_path(),
            "--namespace",
            "acme",
        ])
        .assert()
        .success();

    let root = dir.path().join("my-plugin");
    assert!(root.join("Cargo.toml").is_file());
    assert!(root.join("src/lib.rs").is_file());
    assert!(root.join("moonlit-plugin.toml").is_file());
    assert!(root.join("README.md").is_file());

    let plugin_toml = std::fs::read_to_string(root.join("moonlit-plugin.toml")).unwrap();
    assert!(plugin_toml.contains("namespace = \"acme\""));
    assert!(plugin_toml.contains("license = \"Apache-2.0\""));

    // Buildability proof: the generated crate's native unit test compiles and passes.
    Command::new("cargo")
        .current_dir(&root)
        .args(["test"])
        .assert()
        .success();
}

#[test]
fn new_rejects_an_invalid_name() {
    let dir = tempfile::tempdir().unwrap();
    Command::cargo_bin("moonlit")
        .unwrap()
        .current_dir(dir.path())
        .args(["plugin", "new", "2bad", "--sdk-path", &sdk_path()])
        .assert()
        .code(2);
}

#[test]
fn new_refuses_to_overwrite_existing_dir() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("taken")).unwrap();
    Command::cargo_bin("moonlit")
        .unwrap()
        .current_dir(dir.path())
        .args(["plugin", "new", "taken", "--sdk-path", &sdk_path()])
        .assert()
        .code(2);
}
