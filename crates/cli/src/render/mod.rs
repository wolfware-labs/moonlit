use crate::cli::OutputMode;
use moonlit_engine::pipeline::PipelineEvent;
use std::io::{IsTerminal, stderr};
use std::path::PathBuf;

pub mod json;
pub mod plain;
pub mod pretty;
pub mod summary;

pub struct Header {
    pub version: &'static str,
    pub name: Option<String>,
    pub working_dir: PathBuf,
    pub config_file: String,
    pub stages: Vec<String>,
}

impl Header {
    pub fn new(
        working_dir: PathBuf,
        config_file: String,
        stages: &[String],
        name: Option<String>,
    ) -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION"),
            name,
            working_dir,
            config_file,
            stages: stages.iter().map(|x| x.clone()).collect(),
        }
    }
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
