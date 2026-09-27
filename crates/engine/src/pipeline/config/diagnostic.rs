use crate::pipeline::config::source::ConfigSource;
use crate::pipeline::config::span::Span;
use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
#[error("{message}")]
#[diagnostic(code(moonlit::config))]
pub struct ConfigDiagnostic {
  message: String,
  #[source_code]
  src: NamedSource<String>,
  #[label("{label}")]
  span: Option<SourceSpan>,
  label: String,
}

impl ConfigDiagnostic {
  pub fn invalid_syntax(source: &ConfigSource, info: &str, span: Span) -> ConfigDiagnostic {
    Self::make(source.name, source.yaml, format!("Invalid YAML: {info}"), Some(span), "here")
  }

  pub fn unknown_alias(source: &ConfigSource, span: Span) -> ConfigDiagnostic {
    Self::make(
      source.name,
      source.yaml,
      "Unknown YAML alias: no matching anchor was defined.".to_string(),
      Some(span),
      "undefined alias",
    )
  }

  pub fn expected_mapping(source: &ConfigSource, context: &str, span: Span) -> ConfigDiagnostic {
    Self::make(
      source.name,
      source.yaml,
      format!("Expected a mapping for {context}."),
      Some(span),
      "expected a mapping",
    )
  }

  pub fn expected_sequence(source: &ConfigSource, context: &str, span: Span) -> ConfigDiagnostic {
    Self::make(
      source.name,
      source.yaml,
      format!("Expected a sequence for {context}."),
      Some(span),
      "expected a sequence",
    )
  }

  pub fn expected_string(source: &ConfigSource, context: &str, span: Span) -> ConfigDiagnostic {
    Self::make(
      source.name,
      source.yaml,
      format!("Expected a string value in {context}."),
      Some(span),
      "expected a string",
    )
  }

  pub fn invalid_run(source: &ConfigSource, value: &str, span: Span) -> ConfigDiagnostic {
    Self::make(
      source.name,
      source.yaml,
      format!("'{value}' is not a valid run reference; use the format 'plugin.middleware'."),
      Some(span),
      "expected 'plugin.middleware'",
    )
  }

  pub fn missing_run(source: &ConfigSource, step: &str, span: Span) -> ConfigDiagnostic {
    Self::make(
      source.name,
      source.yaml,
      format!("Step '{step}' is missing a 'run' entry."),
      Some(span),
      "add a 'run: plugin.middleware' entry",
    )
  }

  pub fn invalid_filesystem(source: &ConfigSource, value: &str, span: Span) -> ConfigDiagnostic {
    Self::make(
      source.name,
      source.yaml,
      format!("Invalid filesystem access: {value}. Expected one of: none, read-only, read-write."),
      Some(span),
      "unrecognized filesystem access level",
    )
  }

  pub fn invalid_bool(source: &ConfigSource, field: &str, value: &str, span: Span) -> ConfigDiagnostic {
    Self::make(
      source.name,
      source.yaml,
      format!("Invalid {field} value: {value}. Expected 'true' or 'false'."),
      Some(span),
      "expected 'true' or 'false'",
    )
  }

  pub fn invalid_url(source: &ConfigSource, value: &str, span: Span) -> ConfigDiagnostic {
    Self::make(
      source.name,
      source.yaml,
      format!(
        "Invalid plugin url: {value}. Expected an absolute URL with scheme \
                 'oci', 'file', 'http', or 'https'."
      ),
      Some(span),
      "unsupported or malformed url",
    )
  }

  pub fn missing_url(source: &ConfigSource, plugin: &str, span: Span) -> ConfigDiagnostic {
    Self::make(
      source.name,
      source.yaml,
      format!("Plugin '{plugin}' is missing a 'url' entry."),
      Some(span),
      "add a 'url' entry",
    )
  }

  pub fn no_stages(source: &ConfigSource) -> ConfigDiagnostic {
    Self::make(
      source.name,
      source.yaml,
      "No stages defined. A pipeline needs at least one stage.".to_string(),
      None,
      "",
    )
  }

  pub fn no_plugins(source: &ConfigSource, span: Option<Span>) -> ConfigDiagnostic {
    Self::make(
      source.name,
      source.yaml,
      "No plugins declared. Every step runs a middleware from a plugin, so at least one is required.".to_string(),
      span,
      "define at least one plugin",
    )
  }

  pub fn plugin_not_found(source: &ConfigSource, name: &str, span: Span) -> ConfigDiagnostic {
    Self::make(
      source.name,
      source.yaml,
      format!("No plugin is declared with the alias '{name}'."),
      Some(span),
      "unknown plugin",
    )
  }

  pub fn middleware_not_found(source: &ConfigSource, name: &str, span: Span) -> ConfigDiagnostic {
    Self::make(
      source.name,
      source.yaml,
      format!("The plugin does not export a middleware named '{name}'."),
      Some(span),
      "unknown middleware",
    )
  }

  pub fn duplicate_plugin(source: &ConfigSource, name: &str, span: Span) -> ConfigDiagnostic {
    Self::make(
      source.name,
      source.yaml,
      format!("Duplicate plugin name '{name}'. Plugin names must be unique."),
      Some(span),
      "duplicate plugin name",
    )
  }

  pub fn unknown_key(source: &ConfigSource, key: &str, context: &str, span: Span) -> ConfigDiagnostic {
    Self::make(
      source.name,
      source.yaml,
      format!("Unknown {context} key '{key}'."),
      Some(span),
      "unknown key",
    )
  }

  pub fn duplicate_key(source: &ConfigSource, key: &str, span: Span) -> ConfigDiagnostic {
    Self::make(
      source.name,
      source.yaml,
      format!("Duplicate key '{key}'."),
      Some(span),
      "duplicate key",
    )
  }

  pub fn null_value(source: &ConfigSource, key: &str, expected: &str, span: Span) -> ConfigDiagnostic {
    Self::make(
      source.name,
      source.yaml,
      format!("Key '{key}' expects {expected}, but has no value."),
      Some(span),
      "missing value",
    )
  }

  fn make(config_name: &str, config_content: &str, message: String, span: Option<Span>, label: &str) -> ConfigDiagnostic {
    ConfigDiagnostic {
      message,
      src: NamedSource::new(config_name, config_content.to_string()),
      span: span.map(Span::to_source_span),
      label: label.to_owned(),
    }
  }

  pub fn message(&self) -> &str {
    &self.message
  }

  pub fn span(&self) -> Option<&SourceSpan> {
    self.span.as_ref()
  }
}
