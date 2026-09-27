use crate::plugin::host::HostEventSink;
use crate::plugin::host::child_process::ChildProcess;
use crate::plugin::host::net::AllowlistHooks;
use crate::plugin::wit::moonlit::plugin::host::Host as MoonlitHost;
use crate::plugin::wit::moonlit::plugin::process::{Command, Host as ProcessHost, HostChild, OutputChunk};
use crate::plugin::wit::moonlit::plugin::types::LogLevel;
use std::sync::Arc;
use wasmtime::component::{Resource, ResourceTable};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};
use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpCtxView, WasiHttpView};

pub struct HostState {
  table: ResourceTable,
  wasi: WasiCtx,
  http: WasiHttpCtx,
  hooks: AllowlistHooks,
  events: Arc<dyn HostEventSink>,
  config_view: serde_json::Value,
  exec_allow: globset::GlobSet,
  current_step: String,
}

impl HostState {
  #[must_use]
  pub fn new(
    wasi: WasiCtx,
    hooks: AllowlistHooks,
    events: Arc<dyn HostEventSink>,
    config_view: serde_json::Value,
    exec_allow: globset::GlobSet,
  ) -> Self {
    Self {
      table: ResourceTable::new(),
      wasi,
      http: WasiHttpCtx::new(),
      hooks,
      events,
      config_view,
      exec_allow,
      current_step: String::new(),
    }
  }

  pub fn set_step(&mut self, step: &str) {
    step.clone_into(&mut self.current_step);
  }
}

impl WasiView for HostState {
  fn ctx(&mut self) -> WasiCtxView<'_> {
    WasiCtxView {
      ctx: &mut self.wasi,
      table: &mut self.table,
    }
  }
}

impl WasiHttpView for HostState {
  fn http(&mut self) -> WasiHttpCtxView<'_> {
    WasiHttpCtxView {
      ctx: &mut self.http,
      table: &mut self.table,
      hooks: &mut self.hooks,
    }
  }
}

impl MoonlitHost for HostState {
  async fn log(&mut self, level: LogLevel, message: String) -> wasmtime::Result<()> {
    self.events.log(&self.current_step, level.into(), &message);
    Ok(())
  }

  async fn get_config(&mut self, path: String) -> wasmtime::Result<Option<String>> {
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
  async fn spawn(&mut self, cmd: Command) -> wasmtime::Result<Result<Resource<ChildProcess>, String>> {
    if !self.exec_allow.is_match(&cmd.program) {
      self.events.log(
        &self.current_step,
        crate::plugin::host::LogLevel::Warn,
        &format!("blocked from running '{}' — add it to permissions.exec", cmd.program),
      );
      return Ok(Err(format!("program '{}' not permitted", cmd.program)));
    }
    match ChildProcess::start(&cmd) {
      Ok(child) => Ok(Ok(self.table.push(child)?)),
      Err(e) => Ok(Err(e)),
    }
  }

  async fn run(&mut self, cmd: Command) -> wasmtime::Result<Result<(i32, Vec<OutputChunk>), String>> {
    if !self.exec_allow.is_match(&cmd.program) {
      self.events.log(
        &self.current_step,
        crate::plugin::host::LogLevel::Warn,
        &format!("blocked from running '{}' — add it to permissions.exec", cmd.program),
      );
      return Ok(Err(format!("program '{}' not permitted", cmd.program)));
    }
    let mut child = match ChildProcess::start(&cmd) {
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
  async fn next_line(&mut self, self_: Resource<ChildProcess>) -> wasmtime::Result<Option<OutputChunk>> {
    let child = self.table.get_mut(&self_)?;
    Ok(child.rx.recv().await)
  }

  async fn wait(&mut self, self_: Resource<ChildProcess>) -> wasmtime::Result<i32> {
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

  async fn kill(&mut self, self_: Resource<ChildProcess>) -> wasmtime::Result<()> {
    let child = self.table.get_mut(&self_)?;
    if let Some(tx) = child.kill_tx.take() {
      let _ = tx.send(());
    }
    Ok(())
  }

  async fn drop(&mut self, rep: Resource<ChildProcess>) -> wasmtime::Result<()> {
    let _ = self.table.delete(rep)?;
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::pipeline::config::{FilesystemAccess, Permissions};
  use crate::plugin::host::exec_globset;
  use std::sync::Mutex;
  use wasmtime_wasi::WasiCtxBuilder;

  #[derive(Default)]
  struct RecordingSink {
    logs: Mutex<Vec<(crate::logging::LogLevel, String)>>,
  }

  impl HostEventSink for RecordingSink {
    fn log(&self, _step: &str, level: crate::logging::LogLevel, message: &str) {
      self.logs.lock().unwrap().push((level, message.to_string()));
    }
    fn progress(&self, _step: &str, _message: &str) {}
  }

  fn state(exec: &[&str], sink: &Arc<RecordingSink>) -> HostState {
    let permissions = Permissions {
      network: vec![],
      exec: exec.iter().map(ToString::to_string).collect(),
      env: vec![],
      filesystem: FilesystemAccess::None,
    };
    let events: Arc<dyn HostEventSink> = sink.clone();
    HostState::new(
      WasiCtxBuilder::new().build(),
      AllowlistHooks::new(&permissions, events.clone()),
      events,
      serde_json::json!({}),
      exec_globset(&permissions.exec),
    )
  }

  fn command(program: &str, args: &[&str]) -> Command {
    Command {
      program: program.to_string(),
      args: args.iter().map(ToString::to_string).collect(),
      cwd: None,
      env: vec![],
      stdin: None,
    }
  }

  fn handle(child: &Resource<ChildProcess>) -> Resource<ChildProcess> {
    Resource::new_borrow(child.rep())
  }

  #[tokio::test]
  async fn spawn_of_a_program_outside_the_allowlist_is_refused_and_logged() {
    let sink = Arc::new(RecordingSink::default());
    let mut host = state(&["git"], &sink);

    let result = ProcessHost::spawn(&mut host, command("sh", &[])).await.unwrap();

    assert!(matches!(&result, Err(msg) if msg == "program 'sh' not permitted"));
    let logs = sink.logs.lock().unwrap();
    assert!(
      logs
        .iter()
        .any(|(level, msg)| *level == crate::logging::LogLevel::Warn && msg.contains("permissions.exec"))
    );
  }

  #[tokio::test]
  async fn spawn_and_run_report_programs_that_cannot_start() {
    let sink = Arc::new(RecordingSink::default());
    let mut host = state(&["*"], &sink);

    let spawned = ProcessHost::spawn(&mut host, command("moonlit-no-such-program", &[]))
      .await
      .unwrap();
    let ran = ProcessHost::run(&mut host, command("moonlit-no-such-program", &[]))
      .await
      .unwrap();

    assert!(matches!(&spawned, Err(msg) if msg.starts_with("failed to spawn moonlit-no-such-program")));
    assert!(matches!(&ran, Err(msg) if msg.starts_with("failed to spawn moonlit-no-such-program")));
  }

  #[tokio::test]
  async fn a_spawned_child_streams_lines_and_caches_its_exit_code() {
    let sink = Arc::new(RecordingSink::default());
    let mut host = state(&["sh"], &sink);

    let child = ProcessHost::spawn(&mut host, command("sh", &["-c", "echo first"]))
      .await
      .unwrap()
      .unwrap();
    let line = HostChild::next_line(&mut host, handle(&child)).await.unwrap();
    let end = HostChild::next_line(&mut host, handle(&child)).await.unwrap();
    let code = HostChild::wait(&mut host, handle(&child)).await.unwrap();
    let again = HostChild::wait(&mut host, handle(&child)).await.unwrap();
    HostChild::drop(&mut host, child).await.unwrap();

    assert_eq!(line.map(|chunk| chunk.line), Some("first".to_string()));
    assert!(end.is_none());
    assert_eq!(code, 0);
    assert_eq!(again, 0);
  }

  #[tokio::test]
  async fn a_killed_child_stops_and_a_second_kill_is_harmless() {
    let sink = Arc::new(RecordingSink::default());
    let mut host = state(&["sleep"], &sink);

    let child = ProcessHost::spawn(&mut host, command("sleep", &["30"]))
      .await
      .unwrap()
      .unwrap();
    HostChild::kill(&mut host, handle(&child)).await.unwrap();
    HostChild::kill(&mut host, handle(&child)).await.unwrap();
    let code = HostChild::wait(&mut host, handle(&child)).await.unwrap();

    assert_ne!(code, 0);
  }
}
