mod diagnostic;
mod model;
mod parser;
mod source;
mod span;
mod tree;

pub use crate::pipeline::config::diagnostic::ConfigDiagnostic;
pub use crate::pipeline::config::model::{ConfigMap, ConfigValue, FilesystemAccess, Permissions, PipelineConfig, PluginUrl};
pub use crate::pipeline::config::parser::parse_config;
pub(crate) use crate::pipeline::config::source::ConfigSource;

impl PipelineConfig {
  pub fn add_argument(&mut self, key: &str, value: &str) {
    self.arguments.insert(key.to_owned(), value.to_owned());
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn add_argument_inserts_or_overrides() {
    let yaml =
      "arguments:\n  version: '1.0'\nplugins:\n  - name: tp\n    url: file://tp.wasm\nstages:\n  b:\n    - run: tp.go\n";
    let mut config = parse_config(yaml, "release.yml").expect("valid configuration");
    config.add_argument("version", "2.0");
    config.add_argument("channel", "beta");
    assert_eq!(config.arguments.get("version").map(String::as_str), Some("2.0"));
    assert_eq!(config.arguments.get("channel").map(String::as_str), Some("beta"));
  }
}
