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
fn inspect_json_carries_icon_and_config_schema() {
    let out = Command::cargo_bin("moonlit")
        .unwrap()
        .args(["--output", "json", "plugin", "inspect", FIXTURE])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();

    // ABI 0.2.0: the plugin icon is emitted as a top-level data-URI string.
    let icon = v["icon"].as_str().expect("icon must be a string");
    assert!(
        icon.starts_with("data:image/"),
        "icon must be a data URI; got {icon:.32}"
    );

    // Each middleware's config schema is embedded as a JSON object (not a string).
    let echo = v["middlewares"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["name"] == "echo")
        .expect("echo middleware present");
    let schema = &echo["configSchema"];
    assert!(
        schema.is_object(),
        "configSchema must be an embedded object; got {schema}"
    );
    assert_eq!(
        schema["$schema"], "https://json-schema.org/draft/2020-12/schema",
        "schema must declare the draft 2020-12 dialect"
    );
    assert!(
        schema.pointer("/properties/times").is_some(),
        "echo config schema must expose the `times` property; got {schema}"
    );
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
