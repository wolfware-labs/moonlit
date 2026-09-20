mod config;
mod model;
mod runner;

use crate::pipeline::model::FlatStep;
use crate::plugin::Plugin;
use indexmap::IndexMap;
use std::path::PathBuf;
use std::time::Duration;

pub struct Pipeline {
    plugins: IndexMap<String, Plugin>,
    steps: Vec<FlatStep>,
    working_directory: PathBuf,
    step_timeout: Option<Duration>,
}
