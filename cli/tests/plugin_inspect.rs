use assert_cmd::Command;

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../engine/tests/fixtures/sdk_sample.wasm"
);

/// A plugin whose config validation rejects the empty config; inspect must
/// still describe it. Regression: inspect used to `init({})`, so the
/// `PluginConfig::validate` hook made this fail with a token error.
const REQUIRED_CONFIG_FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../engine/tests/fixtures/github.wasm"
);

#[test]
fn inspect_prints_metadata_and_middlewares() {
    Command::cargo_bin("moonlit")
        .unwrap()
        .args(["plugin", "inspect", FIXTURE])
        .assert()
        .success()
        .stdout(predicates::str::contains("sdk-sample"))
        .stdout(predicates::str::contains("echo"))
        .stdout(predicates::str::contains("fail"));
}

#[test]
fn inspect_json_emits_structured_output() {
    let out = Command::cargo_bin("moonlit")
        .unwrap()
        .args(["--output", "json", "plugin", "inspect", FIXTURE])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["name"], "sdk-sample");
    let names: Vec<_> = v["middlewares"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["name"].as_str().unwrap().to_string())
        .collect();
    assert!(names.contains(&"echo".to_string()));
}

#[test]
fn inspect_succeeds_for_required_config_plugin() {
    Command::cargo_bin("moonlit")
        .unwrap()
        .args(["plugin", "inspect", REQUIRED_CONFIG_FIXTURE])
        .assert()
        .success()
        .stdout(predicates::str::contains("github"))
        .stdout(predicates::str::contains("related-items"))
        .stdout(predicates::str::contains("create-release"))
        .stdout(predicates::str::contains("write-variables"));
}

#[test]
fn inspects_a_component_by_file_ref() {
    // Absolute path to a committed component fixture, expressed as a file:// ref.
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../engine/tests/fixtures/sdk_sample.wasm")
        .canonicalize()
        .unwrap();
    let url = format!("file://{}", fixture.display());
    Command::cargo_bin("moonlit")
        .unwrap()
        .args(["plugin", "inspect", &url, "--output", "plain"])
        .assert()
        .success()
        .stdout(predicates::str::contains("name:"))
        .stdout(predicates::str::contains("sdk-sample"));
}

#[test]
fn inspect_rejects_a_non_component() {
    Command::cargo_bin("moonlit")
        .unwrap()
        .args([
            "plugin",
            "inspect",
            concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"),
        ])
        .assert()
        .code(2);
}
