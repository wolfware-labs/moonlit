use crate::cli::OutputMode;
use moonlit_engine::pipeline::PipelineEvent;
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
