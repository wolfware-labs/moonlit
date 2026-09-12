//! Rendering of the pipeline event stream in three modes: pretty (TTY), plain (CI), json.

pub mod json;
pub mod plain;
pub mod pretty;
pub mod summary;

use moonlit_engine::PipelineEvent;

/// Startup header data (MVP_SPEC §9.4.1). `version` is the CLI's own version.
pub struct Header {
    pub version: &'static str,
    pub name: Option<String>,
    pub working_dir: String,
    pub config_file: String,
    /// Configured stage names (peeked) or the active `-s` filter.
    pub stages: Vec<String>,
}

/// Consumes the event stream. One renderer instance handles a whole run.
pub trait Renderer: Send {
    fn header(&mut self, header: &Header);
    fn handle(&mut self, event: &PipelineEvent);
    fn finish(&mut self);
}

use crate::cli::OutputMode;

/// Choose the effective mode: explicit flag wins; otherwise pretty on a TTY, else plain.
pub fn resolve_mode(opt: Option<OutputMode>, stderr_is_tty: bool) -> OutputMode {
    match opt {
        Some(m) => m,
        None if stderr_is_tty => OutputMode::Pretty,
        None => OutputMode::Plain,
    }
}

use std::io::IsTerminal;

/// Build the renderer for the resolved mode. Pretty/plain go to stderr; json to stdout.
pub fn for_mode(opt: Option<OutputMode>, stderr_is_tty: bool, verbose: bool) -> Box<dyn Renderer> {
    match resolve_mode(opt, stderr_is_tty) {
        OutputMode::Pretty => Box::new(pretty::PrettyRenderer::new(verbose)),
        OutputMode::Plain => Box::new(plain::PlainRenderer::new(std::io::stderr(), verbose)),
        OutputMode::Json => Box::new(json::JsonRenderer::new(std::io::stdout())),
    }
}

/// Whether stderr is a terminal (used to auto-select pretty vs plain).
pub fn stderr_is_tty() -> bool {
    std::io::stderr().is_terminal()
}

#[cfg(test)]
mod mode_tests {
    use super::*;
    use crate::cli::OutputMode;

    #[test]
    fn explicit_mode_always_wins() {
        assert_eq!(
            resolve_mode(Some(OutputMode::Plain), true),
            OutputMode::Plain
        );
        assert_eq!(
            resolve_mode(Some(OutputMode::Pretty), false),
            OutputMode::Pretty
        );
        assert_eq!(resolve_mode(Some(OutputMode::Json), true), OutputMode::Json);
    }

    #[test]
    fn auto_picks_pretty_on_tty_plain_otherwise() {
        assert_eq!(resolve_mode(None, true), OutputMode::Pretty);
        assert_eq!(resolve_mode(None, false), OutputMode::Plain);
    }

    #[test]
    fn factory_constructs_without_panicking() {
        // Smoke: each mode builds a renderer.
        let _ = for_mode(Some(OutputMode::Plain), false, false);
        let _ = for_mode(Some(OutputMode::Json), false, false);
        let _ = for_mode(Some(OutputMode::Pretty), true, true);
    }
}
