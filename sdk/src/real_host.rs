//! The real host: implements `Host` against the generated wit-bindgen imports.
//! wasm-only — the native import stubs abort if called, so this whole module is
//! `cfg(target_arch = "wasm32")`. Binding-call shapes verified against wasm32-wasip2.

use crate::context::{Host, LogLevel};
use crate::process::{ChildHandle, OutputChunk, ProcessCommand, ProcessOutput, StdioStream};

/// The real host, backed by the wit-bindgen imports.
pub struct RealHost;

impl Host for RealHost {
    fn log(&self, level: LogLevel, message: &str) {
        use crate::bindings::moonlit::plugin::types::LogLevel as W;
        let w = match level {
            LogLevel::Debug => W::Debug,
            LogLevel::Info => W::Info,
            LogLevel::Warn => W::Warn,
            LogLevel::Error => W::Error,
        };
        crate::bindings::moonlit::plugin::host::log(w, message);
    }
    fn get_config(&self, path: &str) -> Option<String> {
        crate::bindings::moonlit::plugin::host::get_config(path)
    }
    fn report_progress(&self, message: &str) {
        crate::bindings::moonlit::plugin::host::report_progress(message);
    }
    fn process_run(&self, cmd: &ProcessCommand) -> Result<ProcessOutput, String> {
        let (code, chunks) = crate::bindings::moonlit::plugin::process::run(&to_wit_command(cmd))?;
        Ok(ProcessOutput {
            exit_code: code,
            chunks: chunks.into_iter().map(from_wit_chunk).collect(),
        })
    }
    fn process_spawn(&self, cmd: &ProcessCommand) -> Result<Box<dyn ChildHandle>, String> {
        let child = crate::bindings::moonlit::plugin::process::spawn(&to_wit_command(cmd))?;
        Ok(Box::new(RealChild { child }))
    }
    fn env_var(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }
    fn env_vars(&self) -> Vec<(String, String)> {
        std::env::vars().collect()
    }
}

fn to_wit_command(cmd: &ProcessCommand) -> crate::bindings::moonlit::plugin::process::Command {
    crate::bindings::moonlit::plugin::process::Command {
        program: cmd.program.clone(),
        args: cmd.args.clone(),
        cwd: cmd.cwd.clone(),
        env: cmd.env.clone(),
        stdin: cmd.stdin.clone(),
    }
}

fn from_wit_chunk(c: crate::bindings::moonlit::plugin::process::OutputChunk) -> OutputChunk {
    let stream = match c.stream {
        crate::bindings::moonlit::plugin::process::StdioStream::Stdout => StdioStream::Stdout,
        crate::bindings::moonlit::plugin::process::StdioStream::Stderr => StdioStream::Stderr,
    };
    OutputChunk {
        stream,
        text: c.line,
    }
}

struct RealChild {
    child: crate::bindings::moonlit::plugin::process::Child,
}

impl ChildHandle for RealChild {
    fn next_line(&mut self) -> Option<OutputChunk> {
        self.child.next_line().map(from_wit_chunk)
    }
    fn wait(&mut self) -> i32 {
        self.child.wait()
    }
    fn kill(&mut self) {
        self.child.kill();
    }
}
