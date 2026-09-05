use assert_cmd::Command;

#[test]
fn version_prints_brand_and_license() {
    let out = Command::cargo_bin("moonlit")
        .unwrap()
        .arg("version")
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let expected = format!("Moonlit v{}", env!("CARGO_PKG_VERSION"));
    assert!(stdout.contains(&expected), "stdout: {stdout}");
    assert!(stdout.contains("Wolfware LLC"), "stdout: {stdout}");
    assert!(
        stdout.contains("License: MIT OR Apache-2.0"),
        "stdout: {stdout}"
    );
}

#[test]
fn bare_invocation_prints_help() {
    let out = Command::cargo_bin("moonlit").unwrap().assert().success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("Usage: moonlit"),
        "bare `moonlit` must print help, stdout: {stdout}"
    );
    let banner = format!("Moonlit v{}", env!("CARGO_PKG_VERSION"));
    assert!(
        !stdout.contains(&banner),
        "bare `moonlit` must not print the version banner, stdout: {stdout}"
    );
}
