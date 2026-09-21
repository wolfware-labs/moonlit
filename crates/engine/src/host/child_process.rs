use crate::host::wit::moonlit::plugin::process::{Command, OutputChunk, StdioStream};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, oneshot};

pub struct ChildProc {
    pub rx: mpsc::Receiver<OutputChunk>,
    pub exit_rx: Option<oneshot::Receiver<i32>>,
    pub exit_cached: Option<i32>,
    pub kill_tx: Option<oneshot::Sender<()>>,
}

impl ChildProc {
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

        let mut child = c
            .spawn()
            .map_err(|e| format!("failed to spawn {}: {e}", command.program))?;

        if let (Some(input), Some(mut stdin)) = (command.stdin.clone(), child.stdin.take()) {
            tokio::spawn(async move {
                let _ = stdin.write_all(input.as_bytes()).await;
                let _ = stdin.shutdown().await;
            });
        }

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "no stdout pipe".to_string())?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| "no stderr pipe".to_string())?;

        let (tx, rx) = mpsc::channel::<OutputChunk>(64);
        let (exit_tx, exit_rx) = oneshot::channel::<i32>();
        let (kill_tx, kill_rx) = oneshot::channel::<()>();

        tokio::spawn(Self::reader_task(
            child, stdout, stderr, tx, exit_tx, kill_rx,
        ));

        Ok(ChildProc {
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
