#![cfg(unix)]

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

fn fixture_wasm_url() -> String {
  let p = Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../engine/tests/fixtures/test_plugin.wasm")
    .canonicalize()
    .expect("fixture wasm exists");
  format!("file://{}", p.display())
}

#[test]
fn ctrl_c_cancels_a_running_pipeline() {
  let dir = tempfile::tempdir().unwrap();
  let release = dir.path().join("release.yml");
  std::fs::write(
    &release,
    format!(
      "name: demo\nplugins:\n  - name: tp\n    url: {}\nstages:\n  build:\n    - name: s1\n      run: tp.sleep\n      config:\n        ms: '60000'\n",
      fixture_wasm_url()
    ),
  )
  .unwrap();

  let mut child = Command::new(env!("CARGO_BIN_EXE_moonlit"))
    .args(["run", "--output", "plain", "-f"])
    .arg(&release)
    .stderr(Stdio::piped())
    .stdout(Stdio::null())
    .spawn()
    .unwrap();

  let (tx, rx) = mpsc::channel();
  let stderr = child.stderr.take().unwrap();
  std::thread::spawn(move || {
    for line in BufReader::new(stderr).lines().map_while(Result::ok) {
      let _ = tx.send(line);
    }
  });

  let started = rx
    .iter()
    .find(|line| line.starts_with("Step 1/1"))
    .expect("the step starts before stderr closes");
  assert!(started.contains("s1"));

  let pid = libc::pid_t::try_from(child.id()).unwrap();
  assert_eq!(unsafe { libc::kill(pid, libc::SIGINT) }, 0);

  let status = child.wait().unwrap();
  let rest: Vec<String> = rx
    .recv_timeout(Duration::from_secs(5))
    .into_iter()
    .chain(rx.try_iter())
    .collect();
  assert_eq!(status.code(), Some(4), "stderr after the step started: {rest:?}");
  assert!(rest.iter().any(|l| l.contains("Cancelling")), "{rest:?}");
}
