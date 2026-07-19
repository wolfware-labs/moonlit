//! Hand-written impls of the custom `moonlit:plugin/host` + `/process` interfaces.
//! wasi:* imports come from wasmtime-wasi and are not implemented here.

use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::{mpsc, oneshot};
use wasmtime::component::Resource;

use crate::host::moonlit::plugin::host::Host as MoonlitHost;
use crate::host::moonlit::plugin::process::{
    Command, Host as ProcessHost, HostChild, OutputChunk, StdioStream,
};
use crate::host::moonlit::plugin::types::LogLevel as RawLogLevel;
use crate::host::{ChildProc, HostState};

impl MoonlitHost for HostState {
    async fn log(&mut self, level: RawLogLevel, message: String) -> wasmtime::Result<()> {
        self.events.log(
            &self.current_step,
            crate::host::convert::log_level(level),
            &message,
        );
        Ok(())
    }

    async fn get_config(&mut self, path: String) -> wasmtime::Result<Option<String>> {
        // `:`-separated lookup into the injected config view; serialize the hit as JSON text.
        let mut cur = &self.config_view;
        for seg in path.split(':') {
            match cur.get(seg) {
                Some(next) => cur = next,
                None => return Ok(None),
            }
        }
        Ok(Some(cur.to_string()))
    }

    async fn report_progress(&mut self, message: String) -> wasmtime::Result<()> {
        self.events.progress(&self.current_step, &message);
        Ok(())
    }
}

impl ProcessHost for HostState {
    async fn spawn(
        &mut self,
        cmd: Command,
    ) -> wasmtime::Result<Result<Resource<ChildProc>, String>> {
        if !self.exec_allow.is_match(&cmd.program) {
            return Ok(Err(format!("program '{}' not permitted", cmd.program)));
        }
        match spawn_streaming(&cmd) {
            Ok(child) => Ok(Ok(self.table.push(child)?)),
            Err(e) => Ok(Err(e)),
        }
    }

    async fn run(
        &mut self,
        cmd: Command,
    ) -> wasmtime::Result<Result<(i32, Vec<OutputChunk>), String>> {
        if !self.exec_allow.is_match(&cmd.program) {
            return Ok(Err(format!("program '{}' not permitted", cmd.program)));
        }
        // run-to-completion: spawn, drain all lines, wait.
        let mut child = match spawn_streaming(&cmd) {
            Ok(c) => c,
            Err(e) => return Ok(Err(e)),
        };
        let mut chunks = Vec::new();
        while let Some(line) = child.rx.recv().await {
            chunks.push(line);
        }
        let code = match child.exit_rx.take() {
            Some(rx) => rx.await.unwrap_or(-1),
            None => -1,
        };
        Ok(Ok((code, chunks)))
    }
}

impl HostChild for HostState {
    async fn next_line(
        &mut self,
        self_: Resource<ChildProc>,
    ) -> wasmtime::Result<Option<OutputChunk>> {
        let child = self.table.get_mut(&self_)?;
        Ok(child.rx.recv().await)
    }

    async fn wait(&mut self, self_: Resource<ChildProc>) -> wasmtime::Result<i32> {
        let child = self.table.get_mut(&self_)?;
        if let Some(code) = child.exit_cached {
            return Ok(code);
        }
        let code = match child.exit_rx.take() {
            Some(rx) => rx.await.unwrap_or(-1),
            None => -1,
        };
        child.exit_cached = Some(code);
        Ok(code)
    }

    async fn kill(&mut self, self_: Resource<ChildProc>) -> wasmtime::Result<()> {
        let child = self.table.get_mut(&self_)?;
        if let Some(tx) = child.kill_tx.take() {
            let _ = tx.send(());
        }
        Ok(())
    }

    async fn drop(&mut self, rep: Resource<ChildProc>) -> wasmtime::Result<()> {
        let _ = self.table.delete(rep)?;
        Ok(())
    }
}

/// Spawn a process and stream stdout+stderr live through a channel. The
/// `tokio::process::Child` moves into the reader task; only Send endpoints return.
fn spawn_streaming(cmd: &Command) -> Result<ChildProc, String> {
    let mut c = tokio::process::Command::new(&cmd.program);
    c.args(&cmd.args);
    if let Some(cwd) = &cmd.cwd {
        c.current_dir(cwd);
    }
    for (k, v) in &cmd.env {
        c.env(k, v);
    }
    c.stdin(Stdio::null());
    c.stdout(Stdio::piped());
    c.stderr(Stdio::piped());

    let mut child = c
        .spawn()
        .map_err(|e| format!("failed to spawn {}: {e}", cmd.program))?;
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

    tokio::spawn(reader_task(child, stdout, stderr, tx, exit_tx, kill_rx));

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
