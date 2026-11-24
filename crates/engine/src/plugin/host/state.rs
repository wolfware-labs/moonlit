use crate::plugin::host::HostEventSink;
use crate::plugin::host::child_process::{
  ChildProcess, ChildSandbox, StartError, climbs, is_loader_variable, is_variable_name,
};
use crate::plugin::host::net::AllowlistHooks;
use crate::plugin::permissions::{DeclaredPermission, PermissionKind};
use crate::plugin::wit::moonlit::plugin::host::Host as MoonlitHost;
use crate::plugin::wit::moonlit::plugin::process::{Command, Host as ProcessHost, HostChild, OutputChunk};
use crate::plugin::wit::moonlit::plugin::types::LogLevel;
use std::cmp::Ordering;
use std::fmt;
use std::sync::{Arc, Mutex};
use wasmtime::component::{Resource, ResourceTable};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};
use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpCtxView, WasiHttpView};

/// A limit on a plugin that no grant lifts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
  /// The plugin set a variable that chooses which program or libraries load.
  LoaderVariable,
  /// The plugin set a variable whose name is empty or holds `=` or a NUL.
  VariableName,
  /// The plugin was refused more than [`MAX_DENIALS`] distinct things in one call.
  TooManyDenials,
  /// The plugin asked for a folder outside the working directory.
  Folder,
  /// The program path climbs out of a folder with `..`.
  ProgramPath,
}

/// A sandbox refusal recorded while a plugin ran.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Denial {
  /// The grant that would have allowed it; `Exec` for every refusal under a [`Rule`].
  pub kind: PermissionKind,
  /// What was refused: a host, a program, a variable name or a folder.
  pub target: String,
  /// Set when the refusal comes from a [`Rule`], so no grant would have allowed it.
  pub rule: Option<Rule>,
}

impl Denial {
  /// A refusal for want of a grant of `kind` that covers `target`.
  #[must_use]
  pub fn ungranted(kind: PermissionKind, target: &str) -> Self {
    Self {
      kind,
      target: target.to_string(),
      rule: None,
    }
  }

  /// A refusal under `rule`, which holds whatever the plugin was granted.
  #[must_use]
  pub fn against(rule: Rule, target: &str) -> Self {
    Self {
      kind: PermissionKind::Exec,
      target: target.to_string(),
      rule: Some(rule),
    }
  }

  /// The entry that stands for every refusal past [`MAX_DENIALS`].
  fn too_many() -> Self {
    Self::against(Rule::TooManyDenials, "")
  }
}

impl fmt::Display for Denial {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let target = &self.target;
    match self.rule {
      Some(Rule::LoaderVariable) => write!(
        f,
        "blocked from setting '{target}' for a program it runs \u{2014} no plugin may set it"
      ),
      Some(Rule::Folder) => write!(
        f,
        "blocked from running a program in '{target}' \u{2014} it is outside the working directory"
      ),
      Some(Rule::ProgramPath) => write!(
        f,
        "blocked from running '{target}' \u{2014} a program path may not contain '..'"
      ),
      Some(Rule::VariableName) => write!(
        f,
        "blocked from setting '{target}' for a program it runs \u{2014} it is not a variable name"
      ),
      Some(Rule::TooManyDenials) => write!(
        f,
        "blocked more than {MAX_DENIALS} times in one call \u{2014} the refusals after that are not listed"
      ),
      None => {
        let verb = match self.kind {
          PermissionKind::Network => "connecting to",
          PermissionKind::Exec => "running",
          PermissionKind::Env | PermissionKind::FilesystemRead | PermissionKind::FilesystemWrite => "using",
        };
        write!(
          f,
          "blocked from {verb} '{target}' \u{2014} add it to permissions.{}",
          self.kind.field()
        )?;
        if self.kind == PermissionKind::Exec && target.contains(['/', '\\']) {
          write!(f, " (a wildcard in a grant does not match a path separator)")?;
        }
        Ok(())
      }
    }
  }
}

/// The most distinct refusals kept between two [`DenialLog::take`] calls; one more entry stands for the rest.
pub const MAX_DENIALS: usize = 100;

/// Denials shared between the host state and the HTTP hooks.
#[derive(Clone, Debug, Default)]
pub struct DenialLog(Arc<Mutex<Vec<Denial>>>);

impl DenialLog {
  /// Warns about a refusal on the step's log and keeps it for the step's result. A refusal already kept is not
  /// kept or logged again, so a plugin that retries in a loop cannot grow the log or the step's error without
  /// bound. Past [`MAX_DENIALS`] distinct ones, a single [`Rule::TooManyDenials`] entry stands for the rest: it
  /// fails the call, since a refusal that would have failed it may be among those not kept. The first refusal
  /// past the limit is still logged, so the log names what crossed it.
  pub fn record(&self, events: &dyn HostEventSink, step: &str, denial: Denial) {
    let mut kept = self.0.lock().expect("no thread panics holding the denial log");
    if kept.contains(&denial) {
      return;
    }
    let warn = |denial: &Denial| events.log(step, crate::plugin::host::LogLevel::Warn, &denial.to_string());
    let denial = match kept.len().cmp(&MAX_DENIALS) {
      Ordering::Less => denial,
      Ordering::Equal => {
        warn(&denial);
        Denial::too_many()
      }
      Ordering::Greater => return,
    };
    warn(&denial);
    kept.push(denial);
  }

  /// Removes and returns every refusal recorded so far.
  #[must_use]
  pub fn take(&self) -> Vec<Denial> {
    std::mem::take(&mut *self.0.lock().expect("no thread panics holding the denial log"))
  }
}

pub struct HostState {
  table: ResourceTable,
  wasi: WasiCtx,
  http: WasiHttpCtx,
  hooks: AllowlistHooks,
  events: Arc<dyn HostEventSink>,
  config_view: serde_json::Value,
  exec_allow: globset::GlobSet,
  sandbox: ChildSandbox,
  current_step: String,
  declared: Vec<DeclaredPermission>,
  denials: DenialLog,
}

impl HostState {
  #[must_use]
  pub fn new(
    wasi: WasiCtx,
    hooks: AllowlistHooks,
    events: Arc<dyn HostEventSink>,
    config_view: serde_json::Value,
    exec_allow: globset::GlobSet,
    sandbox: ChildSandbox,
    denials: DenialLog,
  ) -> Self {
    Self {
      table: ResourceTable::new(),
      wasi,
      http: WasiHttpCtx::new(),
      hooks,
      events,
      config_view,
      exec_allow,
      sandbox,
      current_step: String::new(),
      declared: Vec::new(),
      denials,
    }
  }

  /// Removes and returns the sandbox refusals recorded since the last call.
  #[must_use]
  pub fn take_denials(&self) -> Vec<Denial> {
    self.denials.take()
  }

  pub fn set_declared(&mut self, declared: Vec<DeclaredPermission>) {
    self.declared = declared;
  }

  #[must_use]
  pub fn declared(&self) -> &[DeclaredPermission] {
    &self.declared
  }

  pub fn set_step(&mut self, step: &str) {
    step.clone_into(&mut self.current_step);
    self.hooks.set_step(step);
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

impl HostState {
  /// Why `cmd` may not run, recorded as a denial, or `None` when it may.
  fn refuse_exec(&self, cmd: &Command) -> Option<String> {
    let program = &cmd.program;
    if climbs(program) {
      self.deny(Denial::against(Rule::ProgramPath, program));
      return Some(format!("program '{program}' not permitted: its path contains '..'"));
    }
    if !self.exec_allow.is_match(program) {
      self.deny(Denial::ungranted(PermissionKind::Exec, program));
      return Some(format!("program '{program}' not permitted"));
    }
    let refused: Vec<Denial> = cmd.env.iter().filter_map(|(name, _)| self.refuse_variable(name)).collect();
    if refused.is_empty() {
      return None;
    }
    let names: Vec<String> = refused.iter().map(|d| format!("'{}'", d.target)).collect();
    for denial in refused {
      self.deny(denial);
    }
    Some(format!("setting {} for '{program}' is not permitted", names.join(", ")))
  }

  /// Why the plugin may not set `name` for a program it runs, or `None` when it may.
  fn refuse_variable(&self, name: &str) -> Option<Denial> {
    if !is_variable_name(name) {
      Some(Denial::against(Rule::VariableName, name))
    } else if is_loader_variable(name, cfg!(windows)) {
      Some(Denial::against(Rule::LoaderVariable, name))
    } else if self.sandbox.grants(name) {
      None
    } else {
      Some(Denial::ungranted(PermissionKind::Env, name))
    }
  }

  fn deny(&self, denial: Denial) {
    self.denials.record(self.events.as_ref(), &self.current_step, denial);
  }

  /// Starts `cmd` in the plugin's sandbox, after [`HostState::refuse_exec`] let it through.
  fn start(&self, cmd: &Command) -> Result<ChildProcess, String> {
    if let Some(refusal) = self.refuse_exec(cmd) {
      return Err(refusal);
    }
    ChildProcess::start(cmd, &self.sandbox).map_err(|e| match e {
      StartError::Failed(message) => message,
      StartError::OutsideWorkingDirectory(folder) => {
        self.deny(Denial::against(Rule::Folder, &folder));
        format!("running '{}' in '{folder}' is not permitted", cmd.program)
      }
    })
  }
}

impl ProcessHost for HostState {
  async fn spawn(&mut self, cmd: Command) -> wasmtime::Result<Result<Resource<ChildProcess>, String>> {
    match self.start(&cmd) {
      Ok(child) => Ok(Ok(self.table.push(child)?)),
      Err(e) => Ok(Err(e)),
    }
  }

  async fn run(&mut self, cmd: Command) -> wasmtime::Result<Result<(i32, Vec<OutputChunk>), String>> {
    let mut child = match self.start(&cmd) {
      Ok(c) => c,
      Err(e) => return Ok(Err(e)),
    };
    let mut chunks = Vec::new();
    while let Some(line) = child.rx.recv().await {
      chunks.push(line);
    }
    Ok(Ok((child.wait_code().await, chunks)))
  }
}

impl HostChild for HostState {
  async fn next_line(&mut self, self_: Resource<ChildProcess>) -> wasmtime::Result<Option<OutputChunk>> {
    let child = self.table.get_mut(&self_)?;
    Ok(child.rx.recv().await)
  }

  async fn wait(&mut self, self_: Resource<ChildProcess>) -> wasmtime::Result<i32> {
    Ok(self.table.get_mut(&self_)?.wait_code().await)
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
  use crate::plugin::host::build_globset;
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
    state_with_env(exec, &[], sink)
  }

  fn state_with_env(exec: &[&str], env: &[&str], sink: &Arc<RecordingSink>) -> HostState {
    let permissions = Permissions {
      network: vec![],
      exec: exec.iter().map(ToString::to_string).collect(),
      env: env.iter().map(ToString::to_string).collect(),
      filesystem: FilesystemAccess::None,
    };
    let events: Arc<dyn HostEventSink> = sink.clone();
    let denials = DenialLog::default();
    HostState::new(
      WasiCtxBuilder::new().build(),
      AllowlistHooks::new(&permissions, events.clone(), denials.clone()),
      events,
      serde_json::json!({}),
      build_globset(&permissions.exec),
      ChildSandbox::new(
        &std::env::current_dir().unwrap(),
        &permissions.env,
        &std::env::vars().collect::<Vec<_>>(),
      ),
      denials,
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
    assert!(warnings(&sink).iter().any(|msg| msg.contains("permissions.exec")));
  }

  /// What `run` returned, without the output lines.
  fn exit(result: Result<(i32, Vec<OutputChunk>), String>) -> Result<i32, String> {
    result.map(|(code, _)| code)
  }

  fn warnings(sink: &RecordingSink) -> Vec<String> {
    let logs = sink.logs.lock().unwrap();
    logs
      .iter()
      .filter(|(level, _)| *level == crate::logging::LogLevel::Warn)
      .map(|(_, msg)| msg.clone())
      .collect()
  }

  #[tokio::test]
  async fn a_loader_variable_set_by_the_plugin_is_refused_by_spawn_and_run() {
    for name in ["PATH", "LD_PRELOAD", "DYLD_INSERT_LIBRARIES"] {
      let sink = Arc::new(RecordingSink::default());
      let mut host = state_with_env(&["sh"], &["*"], &sink);
      let cmd = || Command {
        env: vec![
          ("HARMLESS".to_string(), "1".to_string()),
          (name.to_string(), "/tmp".to_string()),
        ],
        ..command("sh", &["-c", "true"])
      };

      let spawned = ProcessHost::spawn(&mut host, cmd()).await.unwrap().map(|_| ());
      let ran = ProcessHost::run(&mut host, cmd()).await.unwrap();

      let refusal = format!("setting '{name}' for 'sh' is not permitted");
      assert_eq!(spawned, Err(refusal.clone()));
      assert_eq!(exit(ran), Err(refusal));
      assert_eq!(host.take_denials(), [Denial::against(Rule::LoaderVariable, name)]);
      let expected = format!("blocked from setting '{name}' for a program it runs \u{2014} no plugin may set it");
      assert_eq!(warnings(&sink), [expected]);
    }
  }

  #[tokio::test]
  async fn a_variable_outside_the_env_grant_is_refused_by_spawn_and_run() {
    let sink = Arc::new(RecordingSink::default());
    let mut host = state_with_env(&["sh"], &["MOONLIT_T_GRANTED_*"], &sink);
    let cmd = || Command {
      env: vec![
        ("MOONLIT_T_GRANTED_A".to_string(), "1".to_string()),
        ("GIT_SSH_COMMAND".to_string(), "sh".to_string()),
      ],
      ..command("sh", &["-c", "true"])
    };

    let spawned = ProcessHost::spawn(&mut host, cmd()).await.unwrap().map(|_| ());
    let ran = ProcessHost::run(&mut host, cmd()).await.unwrap();

    let refusal = "setting 'GIT_SSH_COMMAND' for 'sh' is not permitted".to_string();
    assert_eq!(spawned, Err(refusal.clone()));
    assert_eq!(exit(ran), Err(refusal));
    assert_eq!(
      host.take_denials(),
      [Denial::ungranted(PermissionKind::Env, "GIT_SSH_COMMAND")]
    );
    assert_eq!(
      warnings(&sink),
      ["blocked from using 'GIT_SSH_COMMAND' \u{2014} add it to permissions.env"]
    );
  }

  #[tokio::test]
  async fn a_variable_the_env_grant_covers_reaches_the_program() {
    let sink = Arc::new(RecordingSink::default());
    let mut host = state_with_env(&["sh"], &["MOONLIT_T_GRANTED_*"], &sink);
    let cmd = Command {
      env: vec![("MOONLIT_T_GRANTED_A".to_string(), "from-plugin".to_string())],
      ..command("sh", &["-c", "echo \"$MOONLIT_T_GRANTED_A\""])
    };

    let (code, chunks) = ProcessHost::run(&mut host, cmd).await.unwrap().unwrap();

    assert_eq!(code, 0);
    assert_eq!(chunks.iter().map(|c| c.line.as_str()).collect::<Vec<_>>(), ["from-plugin"]);
    assert!(host.take_denials().is_empty());
  }

  #[tokio::test]
  async fn a_loader_variable_is_refused_even_when_the_env_grant_covers_it() {
    let sink = Arc::new(RecordingSink::default());
    let mut host = state_with_env(&["sh"], &["PATH", "LD_*"], &sink);
    let cmd = Command {
      env: vec![("PATH".to_string(), "/tmp".to_string())],
      ..command("sh", &["-c", "true"])
    };

    let result = ProcessHost::run(&mut host, cmd).await.unwrap();

    assert_eq!(exit(result), Err("setting 'PATH' for 'sh' is not permitted".to_string()));
    assert_eq!(host.take_denials(), [Denial::against(Rule::LoaderVariable, "PATH")]);
  }

  #[tokio::test]
  async fn a_program_path_with_a_parent_component_is_refused_whatever_is_granted() {
    let sink = Arc::new(RecordingSink::default());
    let mut host = state(&["/usr/bin/../bin/sh"], &sink);

    let result = ProcessHost::run(&mut host, command("/usr/bin/../bin/sh", &["-c", "true"]))
      .await
      .unwrap();

    assert_eq!(
      exit(result),
      Err("program '/usr/bin/../bin/sh' not permitted: its path contains '..'".to_string())
    );
    assert_eq!(
      host.take_denials(),
      [Denial::against(Rule::ProgramPath, "/usr/bin/../bin/sh")]
    );
    assert_eq!(
      warnings(&sink),
      ["blocked from running '/usr/bin/../bin/sh' \u{2014} a program path may not contain '..'"]
    );
  }

  #[tokio::test]
  async fn a_wildcard_in_a_grant_does_not_cross_a_path_separator() {
    let sink = Arc::new(RecordingSink::default());
    let mut host = state(&["git*", "*"], &sink);

    let nested = ProcessHost::run(&mut host, command("git/sh", &[])).await.unwrap();
    let absolute = ProcessHost::run(&mut host, command("/bin/sh", &["-c", "true"]))
      .await
      .unwrap();
    let bare = ProcessHost::run(&mut host, command("sh", &["-c", "true"])).await.unwrap();

    assert_eq!(exit(nested), Err("program 'git/sh' not permitted".to_string()));
    assert_eq!(exit(absolute), Err("program '/bin/sh' not permitted".to_string()));
    assert!(matches!(bare, Ok((0, _))), "{bare:?}");
    assert_eq!(
      host.take_denials(),
      [
        Denial::ungranted(PermissionKind::Exec, "git/sh"),
        Denial::ungranted(PermissionKind::Exec, "/bin/sh")
      ]
    );
  }

  #[tokio::test]
  async fn a_folder_outside_the_working_directory_is_refused_and_recorded() {
    let sink = Arc::new(RecordingSink::default());
    let mut host = state(&["sh"], &sink);
    let cmd = || Command {
      cwd: Some("..".to_string()),
      ..command("sh", &["-c", "true"])
    };

    let spawned = ProcessHost::spawn(&mut host, cmd()).await.unwrap().map(|_| ());
    let ran = ProcessHost::run(&mut host, cmd()).await.unwrap();

    let refusal = "running 'sh' in '..' is not permitted".to_string();
    assert_eq!(spawned, Err(refusal.clone()));
    assert_eq!(exit(ran), Err(refusal));
    assert_eq!(host.take_denials(), [Denial::against(Rule::Folder, "..")]);
    assert_eq!(
      warnings(&sink),
      ["blocked from running a program in '..' \u{2014} it is outside the working directory"]
    );
  }

  #[tokio::test]
  async fn an_ungranted_program_is_named_before_its_variables() {
    let sink = Arc::new(RecordingSink::default());
    let mut host = state(&["git"], &sink);
    let cmd = Command {
      env: vec![("PATH".to_string(), "/tmp".to_string())],
      ..command("sh", &[])
    };

    let result = ProcessHost::run(&mut host, cmd).await.unwrap();

    assert_eq!(exit(result), Err("program 'sh' not permitted".to_string()));
    assert_eq!(host.take_denials(), [Denial::ungranted(PermissionKind::Exec, "sh")]);
  }

  #[test]
  fn a_repeated_denial_is_kept_and_logged_once() {
    let sink = RecordingSink::default();
    let log = DenialLog::default();

    for _ in 0..3 {
      log.record(&sink, "step", Denial::ungranted(PermissionKind::Exec, "sh"));
    }
    log.record(&sink, "step", Denial::ungranted(PermissionKind::Network, "sh"));
    log.record(&sink, "step", Denial::against(Rule::ProgramPath, "sh"));

    assert_eq!(warnings(&sink).len(), 3);
    assert_eq!(
      log.take(),
      [
        Denial::ungranted(PermissionKind::Exec, "sh"),
        Denial::ungranted(PermissionKind::Network, "sh"),
        Denial::against(Rule::ProgramPath, "sh")
      ]
    );
  }

  #[test]
  fn the_denial_log_stops_growing_at_its_cap_until_it_is_taken() {
    let sink = RecordingSink::default();
    let log = DenialLog::default();
    let host = |n: usize| Denial::ungranted(PermissionKind::Network, &format!("h{n}.example.com"));

    for n in 0..MAX_DENIALS + 5 {
      log.record(&sink, "step", host(n));
    }

    let logged = warnings(&sink);
    assert_eq!(logged.len(), MAX_DENIALS + 2);
    assert_eq!(logged[MAX_DENIALS], host(MAX_DENIALS).to_string());
    assert_eq!(
      logged[MAX_DENIALS + 1],
      "blocked more than 100 times in one call \u{2014} the refusals after that are not listed"
    );
    let kept = log.take();
    assert_eq!(kept.len(), MAX_DENIALS + 1);
    assert_eq!(kept[MAX_DENIALS - 1], host(MAX_DENIALS - 1));
    assert_eq!(kept[MAX_DENIALS], Denial::against(Rule::TooManyDenials, ""));

    log.record(&sink, "step", host(0));
    assert_eq!(log.take(), [host(0)]);
  }

  #[test]
  fn exactly_the_cap_of_denials_has_no_entry_for_the_rest() {
    let sink = RecordingSink::default();
    let log = DenialLog::default();

    for n in 0..MAX_DENIALS {
      log.record(&sink, "step", Denial::ungranted(PermissionKind::Network, &format!("h{n}")));
    }
    log.record(&sink, "step", Denial::ungranted(PermissionKind::Network, "h0"));

    let kept = log.take();
    assert_eq!(kept.len(), MAX_DENIALS);
    assert!(kept.iter().all(|d| d.rule.is_none()));
  }

  #[tokio::test]
  async fn every_refused_variable_of_a_command_is_named_and_recorded() {
    let sink = Arc::new(RecordingSink::default());
    let mut host = state_with_env(&["sh"], &["*"], &sink);
    let other = Arc::new(RecordingSink::default());
    let mut narrow = state_with_env(&["sh"], &["OK_*"], &other);
    let env = |names: &[&str]| names.iter().map(|n| (n.to_string(), "v".to_string())).collect::<Vec<_>>();

    let wide = Command {
      env: env(&["PATH=x", "", "OK_A", "LD_PRELOAD", "GCONV_PATH"]),
      ..command("sh", &["-c", "true"])
    };
    let result = ProcessHost::run(&mut host, wide).await.unwrap();
    assert_eq!(
      exit(result),
      Err("setting 'PATH=x', '', 'LD_PRELOAD', 'GCONV_PATH' for 'sh' is not permitted".to_string())
    );
    assert_eq!(
      host.take_denials(),
      [
        Denial::against(Rule::VariableName, "PATH=x"),
        Denial::against(Rule::VariableName, ""),
        Denial::against(Rule::LoaderVariable, "LD_PRELOAD"),
        Denial::against(Rule::LoaderVariable, "GCONV_PATH"),
      ]
    );
    assert_eq!(
      warnings(&sink)[0],
      "blocked from setting 'PATH=x' for a program it runs \u{2014} it is not a variable name"
    );

    let two = Command {
      env: env(&["APP_VERSION", "OK_A", "DOCKER_HOST"]),
      ..command("sh", &["-c", "true"])
    };
    let result = ProcessHost::run(&mut narrow, two).await.unwrap();
    assert_eq!(
      exit(result),
      Err("setting 'APP_VERSION', 'DOCKER_HOST' for 'sh' is not permitted".to_string())
    );
    assert_eq!(
      narrow.take_denials(),
      [
        Denial::ungranted(PermissionKind::Env, "APP_VERSION"),
        Denial::ungranted(PermissionKind::Env, "DOCKER_HOST"),
      ]
    );
  }

  #[test]
  fn an_ungranted_program_given_by_path_says_wildcards_stop_at_separators() {
    let by_path = Denial::ungranted(PermissionKind::Exec, "/usr/bin/git").to_string();
    let by_name = Denial::ungranted(PermissionKind::Exec, "git").to_string();
    let host = Denial::ungranted(PermissionKind::Network, "example.com/x").to_string();

    assert_eq!(
      by_path,
      "blocked from running '/usr/bin/git' \u{2014} add it to permissions.exec (a wildcard in a grant does not \
       match a path separator)"
    );
    assert_eq!(by_name, "blocked from running 'git' \u{2014} add it to permissions.exec");
    assert!(!host.contains("wildcard"), "{host}");
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
    let again = HostChild::wait(&mut host, handle(&child)).await.unwrap();

    assert_ne!(code, 0);
    assert_eq!(again, code);
    // A signal ends the child with no exit code on unix; Windows ends it with code 1.
    #[cfg(unix)]
    assert_eq!(code, -1);
  }
}
