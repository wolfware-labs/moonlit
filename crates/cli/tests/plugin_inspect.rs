use assert_cmd::Command;

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../engine/tests/fixtures/pdk_sample.wasm");

#[test]
fn inspect_prints_metadata_and_middlewares() {
  Command::cargo_bin("moonlit")
    .unwrap()
    .args(["plugin", "inspect", FIXTURE])
    .assert()
    .success()
    .stdout(predicates::str::contains("pdk-sample"))
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
  assert_eq!(v["name"], "pdk-sample");
  let names: Vec<_> = v["middlewares"]
    .as_array()
    .unwrap()
    .iter()
    .map(|m| m["name"].as_str().unwrap().to_string())
    .collect();
  assert!(names.contains(&"echo".to_string()));
}

#[test]
fn inspect_json_carries_icon_and_io_schemas() {
  let out = Command::cargo_bin("moonlit")
    .unwrap()
    .args(["--output", "json", "plugin", "inspect", FIXTURE])
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let v: serde_json::Value = serde_json::from_slice(&out).unwrap();

  // The plugin icon is emitted as a top-level data-URI string.
  let icon = v["icon"].as_str().expect("icon must be a string");
  assert!(icon.starts_with("data:image/"), "icon must be a data URI; got {icon:.32}");

  // ABI 0.4.0: each middleware's input and output schemas are embedded as JSON
  // objects (not strings).
  let echo = v["middlewares"]
    .as_array()
    .unwrap()
    .iter()
    .find(|m| m["name"] == "echo")
    .expect("echo middleware present");

  let input_schema = &echo["inputSchema"];
  assert!(
    input_schema.is_object(),
    "inputSchema must be an embedded object; got {input_schema}"
  );
  assert_eq!(
    input_schema["$schema"], "https://json-schema.org/draft/2020-12/schema",
    "input schema must declare the draft 2020-12 dialect"
  );
  assert!(
    input_schema.pointer("/properties/times").is_some(),
    "echo input schema must expose the `times` property; got {input_schema}"
  );

  let output_schema = &echo["outputSchema"];
  assert!(
    output_schema.is_object(),
    "outputSchema must be an embedded object; got {output_schema}"
  );
  assert!(
    output_schema.pointer("/properties/plugin_name").is_some(),
    "echo output schema must expose the `plugin_name` property; got {output_schema}"
  );
}

#[test]
fn inspects_a_component_by_file_ref() {
  // Absolute path to a committed component fixture, expressed as a file:// ref.
  let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../engine/tests/fixtures/pdk_sample.wasm")
    .canonicalize()
    .unwrap();
  let url = format!("file://{}", fixture.display());
  Command::cargo_bin("moonlit")
    .unwrap()
    .args(["plugin", "inspect", &url, "--output", "plain"])
    .assert()
    .success()
    .stdout(predicates::str::contains("name:"))
    .stdout(predicates::str::contains("pdk-sample"));
}

#[test]
fn inspect_rejects_a_non_component() {
  Command::cargo_bin("moonlit")
    .unwrap()
    .args(["plugin", "inspect", concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")])
    .assert()
    .code(2);
}

const PERMISSIONS_FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../engine/tests/fixtures/test_plugin.wasm");
const OLD_WORLD_FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../engine/tests/fixtures/test_plugin_v0_3.wasm");

#[test]
fn inspect_json_lists_permissions_and_world() {
  let out = Command::cargo_bin("moonlit")
    .unwrap()
    .args(["--output", "json", "plugin", "inspect", PERMISSIONS_FIXTURE])
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
  assert_eq!(v["world"], "moonlit:plugin@0.4.0");
  assert_eq!(
    v["permissions"],
    serde_json::json!([
      {"kind": "network", "values": ["api.example.com"], "reason": "Calls the example API.", "optional": false},
      {"kind": "exec", "values": ["git"], "reason": "Only for the tag middleware.", "optional": true},
    ])
  );
}

#[test]
fn inspect_rejects_old_world() {
  Command::cargo_bin("moonlit")
    .unwrap()
    .args(["plugin", "inspect", OLD_WORLD_FIXTURE])
    .assert()
    .code(2)
    .stderr(predicates::str::contains("moonlit:plugin@0.3.0"))
    .stderr(predicates::str::contains("moonlit:plugin@0.4.0"));
}

fn inspect_stdout(mode: &str, fixture: &str) -> String {
  let out = Command::cargo_bin("moonlit")
    .unwrap()
    .args(["--output", mode, "plugin", "inspect", fixture])
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
  String::from_utf8(out).unwrap()
}

#[test]
fn inspect_omits_the_permissions_header_when_there_are_none() {
  for mode in ["plain", "pretty"] {
    let stdout = inspect_stdout(mode, FIXTURE);
    assert!(
      !stdout.lines().any(|l| l.trim().eq_ignore_ascii_case("permissions:")),
      "{mode}: {stdout}"
    );
    let last = stdout.lines().last().unwrap_or_default();
    if mode == "plain" {
      assert_eq!(last, "  read-env - read SAMPLE_ENV from the environment", "{stdout}");
    } else {
      assert!(last.starts_with('└'), "{mode}: last line was {last:?}");
    }
  }
}

#[test]
fn inspect_lists_permissions_in_plain_and_pretty() {
  for (mode, header) in [("plain", "permissions:"), ("pretty", "Permissions:")] {
    let stdout = inspect_stdout(mode, PERMISSIONS_FIXTURE);
    assert!(stdout.contains(header), "{mode}: {stdout}");
    assert!(
      stdout.contains("  Connect to the internet (api.example.com): Calls the example API."),
      "{mode}: {stdout}"
    );
    assert!(
      stdout.contains("  Run programs on your machine (git) [optional]: Only for the tag middleware."),
      "{mode}: {stdout}"
    );
  }
}
