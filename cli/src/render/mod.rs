//! Rendering of the pipeline event stream in three modes: pretty (TTY), plain (CI), json.

pub mod json;
pub mod plain;
pub mod summary;
// `pub mod pretty;` and the `for_mode` factory are added in Task 5.

use moonlit_engine::PipelineEvent;

/// Startup header data (MVP_SPEC §9.4.1). `version` is the CLI's own version.
// Not yet constructed: the run command that builds a `Header` and drives a `Renderer` lands in
// a later task.
#[allow(dead_code)]
pub struct Header {
    pub version: &'static str,
    pub name: Option<String>,
    pub working_dir: String,
    pub config_file: String,
    /// Configured stage names (peeked) or the active `-s` filter.
    pub stages: Vec<String>,
}

/// Consumes the event stream. One renderer instance handles a whole run.
// Not yet used: no caller drives a `Renderer` until the run command is wired in a later task.
#[allow(dead_code)]
pub trait Renderer: Send {
    fn header(&mut self, header: &Header);
    fn handle(&mut self, event: &PipelineEvent);
    fn finish(&mut self);
}

use crate::cli::OutputMode;

/// Choose the effective mode: explicit flag wins; otherwise pretty on a TTY, else plain.
// Not yet called: wired into the run command's mode factory in a later task.
#[allow(dead_code)]
pub fn resolve_mode(opt: Option<OutputMode>, stderr_is_tty: bool) -> OutputMode {
    match opt {
        Some(m) => m,
        None if stderr_is_tty => OutputMode::Pretty,
        None => OutputMode::Plain,
    }
}
