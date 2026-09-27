//! Safe, native-testable wrappers over the host `process` capability. `Command`
//! builds a subprocess; `.run()` captures, `.stream()` live-logs via a
//! `LineHandler`, `.spawn()` yields a `Child` for line-by-line control. Non-zero
//! exit is data (`Ok`), not an error; `Err` means the spawn itself failed.

use crate::context::{Host, LogLevel};

/// Which standard stream a line came from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StdioStream {
  Stdout,
  Stderr,
}

/// One line of subprocess output.
#[derive(Clone, Debug)]
pub struct OutputChunk {
  pub stream: StdioStream,
  pub text: String,
}

/// A subprocess specification (plain-Rust DTO crossing the `Host` boundary).
#[derive(Clone, Debug, Default)]
pub struct ProcessCommand {
  pub program: String,
  pub args: Vec<String>,
  pub cwd: Option<String>,
  pub env: Vec<(String, String)>,
  pub stdin: Option<String>,
}

/// Raw result of a run-to-completion (`Host::process_run`).
pub struct ProcessOutput {
  pub exit_code: i32,
  pub chunks: Vec<OutputChunk>,
}

/// A live child process (`Host::process_spawn` return). Object-safe so the real
/// host (wasm `child` resource) and the mock host (canned script) both fit.
pub trait ChildHandle {
  fn next_line(&mut self) -> Option<OutputChunk>;
  fn wait(&mut self) -> i32;
  fn kill(&mut self);
}

/// Captured output of a finished subprocess.
pub struct Output {
  pub exit_code: i32,
  pub stdout: Vec<String>,
  pub stderr: Vec<String>,
}

impl Output {
  #[must_use]
  fn from_raw(raw: ProcessOutput) -> Self {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    for c in raw.chunks {
      match c.stream {
        StdioStream::Stdout => stdout.push(c.text),
        StdioStream::Stderr => stderr.push(c.text),
      }
    }
    Self {
      exit_code: raw.exit_code,
      stdout,
      stderr,
    }
  }
  /// True when the process exited 0.
  #[must_use]
  pub fn success(&self) -> bool {
    self.exit_code == 0
  }
  /// stdout lines joined with '\n'.
  #[must_use]
  pub fn stdout(&self) -> String {
    self.stdout.join("\n")
  }
  /// stderr lines joined with '\n'.
  #[must_use]
  pub fn stderr(&self) -> String {
    self.stderr.join("\n")
  }
}

/// Maps an output line to a log level (or `None` to suppress).
#[must_use = "a line handler does nothing until it is passed to `.stream()`"]
#[allow(clippy::type_complexity)]
pub struct LineHandler {
  classify: Box<dyn Fn(&OutputChunk) -> Option<LogLevel>>,
}

impl LineHandler {
  /// Standard severity heuristic: "error"/"failed" -> Error, "warning" -> Warn,
  /// else Info (case-insensitive on the line text).
  pub fn severity() -> Self {
    Self {
      classify: Box::new(|chunk| {
        let lower = chunk.text.to_ascii_lowercase();
        let level = if lower.contains("error") || lower.contains("failed") {
          LogLevel::Error
        } else if lower.contains("warning") {
          LogLevel::Warn
        } else {
          LogLevel::Info
        };
        Some(level)
      }),
    }
  }
  /// Log every line at a fixed level.
  pub fn at(level: LogLevel) -> Self {
    Self {
      classify: Box::new(move |_| Some(level)),
    }
  }
  /// Capture only; never log.
  pub fn silent() -> Self {
    Self {
      classify: Box::new(|_| None),
    }
  }
  /// Custom classification.
  pub fn custom(f: impl Fn(&OutputChunk) -> Option<LogLevel> + 'static) -> Self {
    Self { classify: Box::new(f) }
  }
  #[must_use]
  fn level_for(&self, chunk: &OutputChunk) -> Option<LogLevel> {
    (self.classify)(chunk)
  }
}

/// Fluent subprocess builder, created via `ctx.command(program)`.
#[must_use = "a command does nothing until `.run()`, `.spawn()` or `.stream()` is called"]
pub struct Command<'a> {
  host: &'a dyn Host,
  cmd: ProcessCommand,
}

impl<'a> Command<'a> {
  pub(crate) fn new(host: &'a dyn Host, program: impl Into<String>) -> Self {
    Self {
      host,
      cmd: ProcessCommand {
        program: program.into(),
        ..Default::default()
      },
    }
  }
  pub fn arg(mut self, a: impl Into<String>) -> Self {
    self.cmd.args.push(a.into());
    self
  }
  pub fn args<I, S>(mut self, args: I) -> Self
  where
    I: IntoIterator<Item = S>,
    S: Into<String>,
  {
    self.cmd.args.extend(args.into_iter().map(Into::into));
    self
  }
  pub fn cwd(mut self, dir: impl Into<String>) -> Self {
    self.cmd.cwd = Some(dir.into());
    self
  }
  pub fn env(mut self, k: impl Into<String>, v: impl Into<String>) -> Self {
    self.cmd.env.push((k.into(), v.into()));
    self
  }
  pub fn envs<I, K, V>(mut self, vars: I) -> Self
  where
    I: IntoIterator<Item = (K, V)>,
    K: Into<String>,
    V: Into<String>,
  {
    self.cmd.env.extend(vars.into_iter().map(|(k, v)| (k.into(), v.into())));
    self
  }
  pub fn stdin(mut self, input: impl Into<String>) -> Self {
    self.cmd.stdin = Some(input.into());
    self
  }

  /// Run to completion, capturing output silently.
  pub fn run(&self) -> Result<Output, String> {
    let raw = self.host.process_run(&self.cmd)?;
    Ok(Output::from_raw(raw))
  }

  /// Spawn and stream: route each line through `handler` to the host log, and
  /// also capture everything into the returned `Output`.
  #[expect(
    clippy::needless_pass_by_value,
    reason = "public SDK API: plugin authors pass a handler inline"
  )]
  pub fn stream(&self, handler: LineHandler) -> Result<Output, String> {
    let mut child = self.host.process_spawn(&self.cmd)?;
    let mut chunks = Vec::new();
    while let Some(chunk) = child.next_line() {
      if let Some(level) = handler.level_for(&chunk) {
        self.host.log(level, &chunk.text);
      }
      chunks.push(chunk);
    }
    let exit_code = child.wait();
    Ok(Output::from_raw(ProcessOutput { exit_code, chunks }))
  }

  /// Spawn for manual line-by-line control.
  pub fn spawn(&self) -> Result<Child<'a>, String> {
    let handle = self.host.process_spawn(&self.cmd)?;
    Ok(Child {
      handle,
      _marker: std::marker::PhantomData,
    })
  }
}

/// A live child process handle.
#[must_use = "a spawned process should be read with `.next_line()` or awaited with `.wait()`"]
pub struct Child<'a> {
  handle: Box<dyn ChildHandle>,
  _marker: std::marker::PhantomData<&'a ()>,
}

impl Child<'_> {
  /// Next output line, or `None` when the process has exited.
  #[must_use]
  pub fn next_line(&mut self) -> Option<OutputChunk> {
    self.handle.next_line()
  }
  /// Wait for exit and return the code.
  pub fn wait(&mut self) -> i32 {
    self.handle.wait()
  }
  /// Kill the process.
  pub fn kill(&mut self) {
    self.handle.kill();
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::Context;
  use crate::context::LogLevel;
  use crate::testing::MockHost;

  fn chunk(stream: StdioStream, text: &str) -> OutputChunk {
    OutputChunk {
      stream,
      text: text.to_string(),
    }
  }

  #[test]
  fn run_captures_output_and_records_command() {
    let host = MockHost::new().with_process_result(
      0,
      vec![chunk(StdioStream::Stdout, "hello"), chunk(StdioStream::Stderr, "note")],
    );
    let ctx = Context::new(&host, "/w".into(), "s".into());
    let out = ctx.command("echo").arg("hello").run().unwrap();
    assert!(out.success());
    assert_eq!(out.stdout(), "hello");
    assert_eq!(out.stderr(), "note");
    let cmds = host.recorded_commands();
    assert_eq!(cmds[0].program, "echo");
    assert_eq!(cmds[0].args, vec!["hello".to_string()]);
  }

  #[test]
  fn stream_routes_lines_through_severity_handler() {
    let host = MockHost::new().with_process_result(
      0,
      vec![
        chunk(StdioStream::Stdout, "building"),
        chunk(StdioStream::Stderr, "ERROR: boom"),
        chunk(StdioStream::Stdout, "warning: hmm"),
      ],
    );
    let ctx = Context::new(&host, "/w".into(), "s".into());
    let out = ctx.command("docker").stream(LineHandler::severity()).unwrap();
    assert_eq!(out.exit_code, 0);
    assert_eq!(
      host.logs(),
      vec![
        (LogLevel::Info, "building".to_string()),
        (LogLevel::Error, "ERROR: boom".to_string()),
        (LogLevel::Warn, "warning: hmm".to_string()),
      ]
    );
  }

  #[test]
  fn spawn_yields_lines_then_exit_code() {
    let host = MockHost::new().with_process_result(3, vec![chunk(StdioStream::Stdout, "a"), chunk(StdioStream::Stdout, "b")]);
    let ctx = Context::new(&host, "/w".into(), "s".into());
    let mut child = ctx.command("sh").spawn().unwrap();
    let mut lines = Vec::new();
    while let Some(c) = child.next_line() {
      lines.push(c.text);
    }
    assert_eq!(lines, vec!["a".to_string(), "b".to_string()]);
    assert_eq!(child.wait(), 3);
  }

  #[test]
  fn spawn_failure_is_err_not_panic() {
    let host = MockHost::new().with_process_error("program 'x' not permitted");
    let ctx = Context::new(&host, "/w".into(), "s".into());
    let e = ctx.command("x").run().err().expect("expected spawn failure");
    assert!(e.contains("not permitted"), "got: {e}");
  }

  #[test]
  fn spawn_and_stream_failures_are_err() {
    let host = MockHost::new()
      .with_process_error("spawn denied")
      .with_process_error("stream denied");
    let ctx = Context::new(&host, "/w".into(), "s".into());
    assert_eq!(ctx.command("x").spawn().err().expect("spawn error"), "spawn denied");
    assert_eq!(
      ctx.command("x").stream(LineHandler::silent()).err().expect("stream error"),
      "stream denied"
    );
  }

  #[test]
  fn builder_records_args_cwd_env_and_stdin() {
    let host = MockHost::new().with_process_result(0, vec![]);
    let ctx = Context::new(&host, "/w".into(), "s".into());
    let out = ctx
      .command("git")
      .arg("commit")
      .args(["-m", "msg"])
      .cwd("/repo")
      .env("A", "1")
      .envs([("B", "2"), ("C", "3")])
      .stdin("input")
      .run()
      .unwrap();
    assert!(out.success());
    let cmd = &host.recorded_commands()[0];
    assert_eq!(cmd.args, vec!["commit", "-m", "msg"]);
    assert_eq!(cmd.cwd.as_deref(), Some("/repo"));
    assert_eq!(
      cmd.env,
      vec![
        ("A".to_string(), "1".to_string()),
        ("B".to_string(), "2".to_string()),
        ("C".to_string(), "3".to_string()),
      ]
    );
    assert_eq!(cmd.stdin.as_deref(), Some("input"));
  }

  #[test]
  fn fixed_level_handler_logs_every_line_at_that_level() {
    let host = MockHost::new().with_process_result(1, vec![chunk(StdioStream::Stdout, "a"), chunk(StdioStream::Stderr, "b")]);
    let ctx = Context::new(&host, "/w".into(), "s".into());
    let out = ctx.command("x").stream(LineHandler::at(LogLevel::Debug)).unwrap();
    assert!(!out.success());
    assert_eq!(
      host.logs(),
      vec![(LogLevel::Debug, "a".to_string()), (LogLevel::Debug, "b".to_string())]
    );
  }

  #[test]
  fn custom_handler_classifies_by_stream() {
    let host =
      MockHost::new().with_process_result(0, vec![chunk(StdioStream::Stdout, "out"), chunk(StdioStream::Stderr, "err")]);
    let ctx = Context::new(&host, "/w".into(), "s".into());
    let handler = LineHandler::custom(|c| (c.stream == StdioStream::Stderr).then_some(LogLevel::Warn));
    let out = ctx.command("x").stream(handler).unwrap();
    assert_eq!(out.stdout(), "out");
    assert_eq!(out.stderr(), "err");
    assert_eq!(host.logs(), vec![(LogLevel::Warn, "err".to_string())]);
  }

  #[test]
  fn spawned_child_can_be_killed() {
    let host = MockHost::new().with_process_result(0, vec![chunk(StdioStream::Stdout, "a")]);
    let ctx = Context::new(&host, "/w".into(), "s".into());
    let mut child = ctx.command("sleep").spawn().unwrap();
    child.kill();
    assert_eq!(child.wait(), 0);
  }

  #[test]
  fn silent_handler_suppresses_logs() {
    let host = MockHost::new().with_process_result(0, vec![chunk(StdioStream::Stdout, "quiet")]);
    let ctx = Context::new(&host, "/w".into(), "s".into());
    let out = ctx.command("echo").stream(LineHandler::silent()).unwrap();
    assert_eq!(out.stdout(), "quiet");
    assert!(host.logs().is_empty());
  }
}
