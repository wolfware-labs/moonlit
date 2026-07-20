use assert_cmd::Command;

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../engine/tests/fixtures/sdk_sample.wasm"
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
