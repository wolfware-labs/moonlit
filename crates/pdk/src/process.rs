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
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
    /// stdout lines joined with '\n'.
    pub fn stdout(&self) -> String {
        self.stdout.join("\n")
    }
    /// stderr lines joined with '\n'.
    pub fn stderr(&self) -> String {
        self.stderr.join("\n")
    }
}

/// Maps an output line to a log level (or `None` to suppress).
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
        Self {
            classify: Box::new(f),
        }
    }
    fn level_for(&self, chunk: &OutputChunk) -> Option<LogLevel> {
        (self.classify)(chunk)
    }
}

/// Fluent subprocess builder, created via `ctx.command(program)`.
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
    #[must_use]
    pub fn arg(mut self, a: impl Into<String>) -> Self {
        self.cmd.args.push(a.into());
        self
    }
    #[must_use]
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.cmd.args.extend(args.into_iter().map(Into::into));
        self
    }
    #[must_use]
    pub fn cwd(mut self, dir: impl Into<String>) -> Self {
        self.cmd.cwd = Some(dir.into());
        self
    }
    #[must_use]
    pub fn env(mut self, k: impl Into<String>, v: impl Into<String>) -> Self {
        self.cmd.env.push((k.into(), v.into()));
        self
    }
    #[must_use]
    pub fn envs<I, K, V>(mut self, vars: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        self.cmd
            .env
            .extend(vars.into_iter().map(|(k, v)| (k.into(), v.into())));
        self
    }
    #[must_use]
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
pub struct Child<'a> {
    handle: Box<dyn ChildHandle>,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl Child<'_> {
    /// Next output line, or `None` when the process has exited.
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
    use crate::context::LogLevel;
    use crate::testing::MockHost;
    use crate::Context;

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
            vec![
                chunk(StdioStream::Stdout, "hello"),
                chunk(StdioStream::Stderr, "note"),
            ],
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
        let out = ctx
            .command("docker")
            .stream(LineHandler::severity())
            .unwrap();
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
        let host = MockHost::new().with_process_result(
            3,
            vec![
                chunk(StdioStream::Stdout, "a"),
                chunk(StdioStream::Stdout, "b"),
            ],
        );
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
        match ctx.command("x").run() {
            Ok(_) => panic!("expected spawn failure"),
            Err(e) => assert!(e.contains("not permitted"), "got: {e}"),
        }
    }

    #[test]
    fn silent_handler_suppresses_logs() {
        let host =
            MockHost::new().with_process_result(0, vec![chunk(StdioStream::Stdout, "quiet")]);
        let ctx = Context::new(&host, "/w".into(), "s".into());
        let out = ctx.command("echo").stream(LineHandler::silent()).unwrap();
        assert_eq!(out.stdout(), "quiet");
        assert!(host.logs().is_empty());
    }
}
