//! Black-box: `moonlit run` drives the slack plugin's `send-notification` via a file://
//! ref. A blank channel hits the pinned guard, which fails before any HTTP call —
//! deterministic and network-free — asserting the CLI surfaces the frozen failure and
//! exits non-zero.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use predicates::str::contains;
use tempfile::tempdir;

fn wasm_url() -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../engine/tests/fixtures/slack.wasm")
        .canonicalize()
        .expect("slack.wasm fixture exists (regenerate via plugins/slack/README.md)");
    format!("file://{}", p.display())
}

#[test]
fn moonlit_run_slack_blank_channel_fails() {
    let dir = tempdir().unwrap();
    let yaml = format!(
        "name: demo\n\
         plugins:\n\
         \x20 - name: slack\n\
         \x20   url: {url}\n\
         \x20   config:\n\
         \x20     token: xoxb-dummy\n\
         \x20   permissions:\n\
         \x20     network: [slack.com]\n\
         stages:\n\
         \x20 release:\n\
         \x20   - name: notify\n\
         \x20     run: slack.send-notification\n\
         \x20     config:\n\
         \x20       message: hello\n",
        url = wasm_url()
    );
    fs::write(dir.path().join("release.yml"), &yaml).unwrap();

    Command::cargo_bin("moonlit")
        .unwrap()
        .args(["run", "--output", "plain", "-w"])
        .arg(dir.path())
        .assert()
        .failure()
        .stderr(contains("No Slack channel provided for notification."));
}
