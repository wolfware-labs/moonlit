use assert_cmd::Command;

#[test]
fn version_prints_brand_and_license() {
    let out = Command::cargo_bin("moonlit")
        .unwrap()
        .arg("version")
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("Moonlit v0.1.0"), "stdout: {stdout}");
    assert!(stdout.contains("Wolfware LLC"), "stdout: {stdout}");
    assert!(stdout.contains("License: Elastic-2.0"), "stdout: {stdout}");
}

#[test]
fn bare_invocation_defaults_to_version() {
    let out = Command::cargo_bin("moonlit").unwrap().assert().success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("Moonlit v0.1.0"), "stdout: {stdout}");
}
