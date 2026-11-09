use crate::pipeline::config::ConfigDiagnostic;
use crate::sanitize::{display_safe, display_safe_text};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IncompatiblePlugin {
  pub name: String,
  pub url: String,
  pub plugin_world: Option<String>,
  pub engine_world: String,
}

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum PipelineError {
  #[error(transparent)]
  #[diagnostic(transparent)]
  Config(#[from] ConfigDiagnostic),

  #[error("failed to load plugin '{plugin}': {message}")]
  #[diagnostic(code(moonlit::pipeline::plugin_load))]
  PluginLoad { plugin: String, message: String },

  #[error("{} plugin(s) target a different plugin contract than this CLI ({})", .0.len(), .0.first().map_or("", |p| p.engine_world.as_str()))]
  #[diagnostic(code(moonlit::pipeline::incompatible_plugins))]
  IncompatiblePlugins(Vec<IncompatiblePlugin>),

  #[error("release file does not grant what its plugins need")]
  #[diagnostic(code(moonlit::pipeline::missing_grants))]
  MissingGrants(#[related] Vec<ConfigDiagnostic>),

  #[error("pipeline execution failed: {0}")]
  #[diagnostic(code(moonlit::pipeline::execution))]
  Execution(String),

  #[error(transparent)]
  #[diagnostic(code(moonlit::pipeline::internal))]
  Internal(#[from] anyhow::Error),
}

impl PipelineError {
  /// The same error with the plugin-sourced text escaped so it can be printed to a terminal.
  #[must_use]
  pub fn for_terminal(self) -> Self {
    match self {
      PipelineError::PluginLoad { plugin, message } => PipelineError::PluginLoad {
        plugin: display_safe(&plugin),
        message: display_safe_text(&message),
      },
      PipelineError::Execution(message) => PipelineError::Execution(display_safe_text(&message)),
      other => other,
    }
  }

  #[must_use]
  pub fn exit_code(&self) -> i32 {
    match self {
      PipelineError::Config(_) | PipelineError::MissingGrants(_) => 2,
      PipelineError::PluginLoad { .. } | PipelineError::IncompatiblePlugins(_) => 3,
      PipelineError::Execution(_) => 4,
      PipelineError::Internal(_) => 1,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::pipeline::config::parse_config;

  #[test]
  fn every_variant_maps_to_its_exit_code() {
    let config = PipelineError::from(parse_config("- a\n", "release.yml").unwrap_err());
    let load = PipelineError::PluginLoad {
      plugin: "p".to_string(),
      message: "m".to_string(),
    };
    let execution = PipelineError::Execution("x".to_string());
    let internal = PipelineError::from(anyhow::anyhow!("boom"));

    assert_eq!(config.exit_code(), 2);
    assert_eq!(load.exit_code(), 3);
    assert_eq!(incompatible_plugins().exit_code(), 3);
    assert_eq!(execution.exit_code(), 4);
    assert_eq!(internal.exit_code(), 1);
  }

  fn incompatible_plugins() -> PipelineError {
    let plugin = |name: &str| IncompatiblePlugin {
      name: name.to_string(),
      url: format!("file:///{name}.wasm"),
      plugin_world: Some("moonlit:plugin@0.3.0".to_string()),
      engine_world: "moonlit:plugin@0.4.0".to_string(),
    };
    PipelineError::IncompatiblePlugins(vec![plugin("a"), plugin("b")])
  }

  #[test]
  fn incompatible_plugins_message_counts_plugins_and_names_the_engine_world() {
    assert_eq!(
      incompatible_plugins().to_string(),
      "2 plugin(s) target a different plugin contract than this CLI (moonlit:plugin@0.4.0)"
    );
  }

  #[test]
  fn messages_name_the_failing_piece() {
    let load = PipelineError::PluginLoad {
      plugin: "git".to_string(),
      message: "not found".to_string(),
    };
    assert_eq!(load.to_string(), "failed to load plugin 'git': not found");
    assert_eq!(
      PipelineError::Execution("halted".to_string()).to_string(),
      "pipeline execution failed: halted"
    );
  }

  #[test]
  fn for_terminal_escapes_plugin_text_and_keeps_lines() {
    let load = PipelineError::PluginLoad {
      plugin: "p\n\u{1b}]0;x".to_string(),
      message: "init failed\n\u{1b}[2K".to_string(),
    };
    assert_eq!(
      load.for_terminal().to_string(),
      "failed to load plugin 'p\\x0a\\x1b]0;x': init failed\n\\x1b[2K"
    );
    let execution = PipelineError::Execution("boom\r\u{1b}[1A".to_string());
    assert_eq!(
      execution.for_terminal().to_string(),
      "pipeline execution failed: boom\\x0d\\x1b[1A"
    );
    assert_eq!(PipelineError::from(anyhow::anyhow!("x")).for_terminal().to_string(), "x");
  }
}
