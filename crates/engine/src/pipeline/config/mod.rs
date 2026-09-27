mod diagnostic;
mod model;
mod parser;
mod span;
mod tree;
mod source;

pub use crate::pipeline::config::diagnostic::ConfigDiagnostic;
pub use crate::pipeline::config::model::{ConfigMap, ConfigValue, FilesystemAccess, Permissions, PipelineConfig, PluginUrl};
pub use crate::pipeline::config::parser::parse_config;
pub(crate) use crate::pipeline::config::source::ConfigSource;

impl PipelineConfig {
  pub fn add_argument(&mut self, key: &str, value: &str) {
    self.arguments.insert(key.to_owned(), value.to_owned());
  }
}
