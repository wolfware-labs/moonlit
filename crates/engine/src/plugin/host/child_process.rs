use crate::plugin::wit::moonlit::plugin::process::{Command, OutputChunk, StdioStream};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, oneshot};

pub struct ChildProcess {
  pub rx: mpsc::Receiver<OutputChunk>,
  pub exit_rx: Option<oneshot::Receiver<i32>>,
  pub exit_cached: Option<i32>,
  pub kill_tx: Option<oneshot::Sender<()>>,
}

impl ChildProcess {
  pub fn start(command: &Command) -> Result<Self, String> {
    let mut c = tokio::process::Command::new(&command.program);
    c.args(&command.args);
    if let Some(cwd) = &command.cwd {
      c.current_dir(cwd);
    }
    for (k, v) in &command.env {
      c.env(k, v);
    }
    c.stdin(if command.stdin.is_some() {
      Stdio::piped()
    } else {
      Stdio::null()
    });
    c.stdout(Stdio::piped());
    c.stderr(Stdio::piped());

    let mut child = c.spawn().map_err(|e| format!("failed to spawn {}: {e}", command.program))?;

    if let (Some(input), Some(mut stdin)) = (command.stdin.clone(), child.stdin.take()) {
      tokio::spawn(async move {
        let _ = stdin.write_all(input.as_bytes()).await;
        let _ = stdin.shutdown().await;
      });
    }

    let stdout = child.stdout.take().ok_or_else(|| "no stdout pipe".to_string())?;
    let stderr = child.stderr.take().ok_or_else(|| "no stderr pipe".to_string())?;

    let (tx, rx) = mpsc::channel::<OutputChunk>(64);
    let (exit_tx, exit_rx) = oneshot::channel::<i32>();
    let (kill_tx, kill_rx) = oneshot::channel::<()>();

    tokio::spawn(Self::reader_task(child, stdout, stderr, tx, exit_tx, kill_rx));

    Ok(ChildProcess {
      rx,
      exit_rx: Some(exit_rx),
      exit_cached: None,
      kill_tx: Some(kill_tx),
    })
  }

  async fn reader_task(
    mut child: tokio::process::Child,
    stdout: tokio::process::ChildStdout,
    stderr: tokio::process::ChildStderr,
    tx: mpsc::Sender<OutputChunk>,
    exit_tx: oneshot::Sender<i32>,
    mut kill_rx: oneshot::Receiver<()>,
  ) {
    let mut out = BufReader::new(stdout).lines();
    let mut err = BufReader::new(stderr).lines();
    let mut out_done = false;
    let mut err_done = false;

    while !(out_done && err_done) {
      tokio::select! {
        biased;
        _ = &mut kill_rx => { let _ = child.start_kill(); break; }
        line = out.next_line(), if !out_done => match line {
          Ok(Some(l)) => {
              if tx.send(OutputChunk { stream: StdioStream::Stdout, line: l }).await.is_err() { break; }
          }
          _ => out_done = true,
        },
        line = err.next_line(), if !err_done => match line {
          Ok(Some(l)) => {
              if tx.send(OutputChunk { stream: StdioStream::Stderr, line: l }).await.is_err() { break; }
          }
          _ => err_done = true,
        },
      }
    }

    drop(tx);
    let code = child.wait().await.ok().and_then(|s| s.code()).unwrap_or(-1);
    let _ = exit_tx.send(code);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  async fn collect(mut child: ChildProcess) -> (Vec<OutputChunk>, i32) {
    let mut chunks = Vec::new();
    while let Some(chunk) = child.rx.recv().await {
      chunks.push(chunk);
    }
    let code = child.exit_rx.take().unwrap().await.unwrap();
    (chunks, code)
  }

  #[tokio::test]
  async fn passes_stdin_env_and_cwd_and_separates_stderr() {
    let dir = tempfile::tempdir().unwrap();
    let command = Command {
      program: "sh".to_string(),
      args: vec![
        "-c".to_string(),
        "cat; echo \"$MOONLIT_CHILD_VAR\"; echo oops >&2; ls".to_string(),
      ],
      cwd: Some(dir.path().display().to_string()),
      env: vec![("MOONLIT_CHILD_VAR".to_string(), "from-env".to_string())],
      stdin: Some("from-stdin\n".to_string()),
    };
    std::fs::write(dir.path().join("marker.txt"), b"").unwrap();

    let (chunks, code) = collect(ChildProcess::start(&command).unwrap()).await;

    let stdout: Vec<&str> = chunks
      .iter()
      .filter(|c| matches!(c.stream, StdioStream::Stdout))
      .map(|c| c.line.as_str())
      .collect();
    let stderr: Vec<&str> = chunks
      .iter()
      .filter(|c| matches!(c.stream, StdioStream::Stderr))
      .map(|c| c.line.as_str())
      .collect();
    assert_eq!(code, 0);
    assert!(stdout.contains(&"from-stdin"), "{stdout:?}");
    assert!(stdout.contains(&"from-env"), "{stdout:?}");
    assert!(stdout.contains(&"marker.txt"), "{stdout:?}");
    assert_eq!(stderr, vec!["oops"]);
  }

  #[test]
  fn a_missing_program_fails_to_start() {
    let command = Command {
      program: "moonlit-no-such-program".to_string(),
      args: vec![],
      cwd: None,
      env: vec![],
      stdin: None,
    };

    let result = ChildProcess::start(&command);

    assert!(matches!(&result, Err(msg) if msg.starts_with("failed to spawn moonlit-no-such-program")));
  }
}
