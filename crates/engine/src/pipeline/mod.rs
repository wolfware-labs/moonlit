pub mod config;
mod data;
mod error;
mod expr;
mod loader;
pub mod manifest;
mod model;
mod runner;

pub(crate) use crate::pipeline::data::PipelineData;
pub use crate::pipeline::error::PipelineError;
use crate::pipeline::model::FlatStep;
pub use crate::pipeline::model::{PipelineEvent, PipelineOptions, PipelineSummary, StepResult};
use crate::plugin::PluginInstance;
use indexmap::IndexMap;
use std::path::PathBuf;
use std::time::Duration;

pub struct Pipeline {
  plugins: IndexMap<String, PluginInstance>,
  steps: Vec<FlatStep>,
  working_directory: PathBuf,
  step_timeout: Option<Duration>,
  data: PipelineData,
}

impl Pipeline {
  pub fn step_count(&self) -> usize {
    self.steps.len()
  }

  pub fn plugin_names(&self) -> Vec<&str> {
    self.plugins.keys().map(String::as_str).collect()
  }
}
