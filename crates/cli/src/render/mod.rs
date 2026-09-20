use crate::cli::OutputMode;
use std::io::{IsTerminal, stderr};

pub mod json;
pub mod plain;
pub mod pretty;
pub mod summary;

pub struct Header {
    pub version: &'static str,
    pub name: Option<String>,
    pub working_dir: String,
    pub config_file: String,
    pub stages: Vec<String>,
}

pub trait Renderer: Send {
    fn header(&mut self, header: &Header);
    fn handle(&mut self, event: &PipelineEvent);
    fn finish(&mut self);
}

pub fn resolve_mode(opt: Option<OutputMode>) -> OutputMode {
    match opt {
        Some(m) => m,
        None if stderr().is_terminal() => OutputMode::Pretty,
        None => OutputMode::Plain,
    }
}

pub fn for_mode(opt: Option<OutputMode>, verbose: bool) -> Box<dyn Renderer> {
    match resolve_mode(opt) {
        OutputMode::Pretty => Box::new(pretty::PrettyRenderer::new(verbose)),
        OutputMode::Plain => Box::new(plain::PlainRenderer::new(stderr(), verbose)),
        OutputMode::Json => Box::new(json::JsonRenderer::new(std::io::stdout())),
    }
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
