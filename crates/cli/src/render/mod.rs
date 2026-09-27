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
  #[must_use]
  pub fn new(working_dir: PathBuf, config_file: String, stages: &[String], name: Option<String>) -> Self {
    Self {
      version: env!("CARGO_PKG_VERSION"),
      name,
      working_dir,
      config_file,
      stages: stages.to_vec(),
    }
  }
}

pub trait Renderer: Send {
  fn header(&mut self, header: &Header);
  fn handle(&mut self, event: &PipelineEvent);
  fn finish(&mut self);
}

#[must_use]
pub fn resolve_mode(opt: Option<OutputMode>) -> OutputMode {
  match opt {
    Some(m) => m,
    None if stderr().is_terminal() => OutputMode::Pretty,
    None => OutputMode::Plain,
  }
}

#[must_use]
pub fn for_mode(opt: Option<OutputMode>, verbose: bool) -> Box<dyn Renderer> {
  match resolve_mode(opt) {
    OutputMode::Pretty => Box::new(pretty::PrettyRenderer::new(verbose)),
    OutputMode::Plain => Box::new(plain::PlainRenderer::new(stderr(), verbose)),
    OutputMode::Json => Box::new(json::JsonRenderer::new(std::io::stdout())),
  }
}

#[cfg(test)]
pub(crate) mod fixtures {
  use super::Header;
  use moonlit_engine::pipeline::{PipelineSummary, StepResult};
  use std::path::PathBuf;
  use std::time::Duration;

  pub fn header(name: Option<&str>, stages: &[&str]) -> Header {
    let stages: Vec<String> = stages.iter().map(ToString::to_string).collect();
    Header::new(
      PathBuf::from("work"),
      "release.yml".to_string(),
      &stages,
      name.map(ToString::to_string),
    )
  }

  pub fn step(name: &str, successful: bool, skipped: bool, error: Option<&str>, warnings: &[&str]) -> StepResult {
    StepResult {
      name: name.to_string(),
      successful,
      skipped,
      duration: Duration::from_millis(1500),
      error_message: error.map(ToString::to_string),
      warnings: warnings.iter().map(ToString::to_string).collect(),
    }
  }

  pub fn summary(steps: Vec<StepResult>, successful: bool) -> PipelineSummary {
    PipelineSummary {
      steps,
      successful,
      halted: false,
      total_duration: Duration::from_millis(2500),
      warnings: Vec::new(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn explicit_modes_are_kept() {
    assert_eq!(resolve_mode(Some(OutputMode::Json)), OutputMode::Json);
    assert_eq!(resolve_mode(Some(OutputMode::Pretty)), OutputMode::Pretty);
    assert_eq!(resolve_mode(Some(OutputMode::Plain)), OutputMode::Plain);
  }

  #[test]
  fn every_mode_builds_a_renderer() {
    for mode in [OutputMode::Json, OutputMode::Plain, OutputMode::Pretty] {
      let mut renderer = for_mode(Some(mode), false);
      renderer.finish();
    }
  }

  #[test]
  fn header_carries_the_crate_version() {
    let h = fixtures::header(Some("demo"), &["build"]);
    assert_eq!(h.version, env!("CARGO_PKG_VERSION"));
    assert_eq!(h.stages, vec!["build".to_string()]);
  }
}
