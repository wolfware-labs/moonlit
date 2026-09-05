//! Span-rich configuration diagnostics.
//!
//! [`ConfigDiagnostic`] is the exit-code-2 error class (§7.2 `EngineError::Config`). It carries the
//! source YAML so the CLI can render a labeled snippet later (§9.4.5); Phase 2 does not install a
//! renderer. Diagnostics are built through [`Source`], which threads the YAML text and file label
//! through the config stages.

use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

use crate::config::model::Span;

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
    /// The human-readable message (exact wording is fixed in the [`Source`] constructors and
    /// asserted verbatim by tests).
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The labeled span, if this diagnostic points at a source location.
    pub fn span(&self) -> Option<&SourceSpan> {
        self.span.as_ref()
    }
}

/// The source under diagnosis, threaded through the config stages. `Copy` so stages pass it freely.
#[derive(Clone, Copy)]
pub struct Source<'a> {
    pub yaml: &'a str,
    pub name: &'a str,
}

impl<'a> Source<'a> {
    pub fn new(yaml: &'a str, name: &'a str) -> Self {
        Self { yaml, name }
    }

    fn make(&self, message: String, span: Option<Span>, label: &str) -> ConfigDiagnostic {
        ConfigDiagnostic {
            message,
            src: NamedSource::new(self.name, self.yaml.to_owned()),
            span: span.map(Span::to_source_span),
            label: label.to_owned(),
        }
    }

    /// A YAML scanner/syntax error at `span`.
    pub fn syntax(&self, info: &str, span: Span) -> ConfigDiagnostic {
        self.make(format!("Invalid YAML: {info}"), Some(span), "here")
    }

    /// A `*alias` referring to an anchor that was never defined.
    pub fn unknown_alias(&self, span: Span) -> ConfigDiagnostic {
        self.make(
            "Unknown YAML alias: no matching anchor was defined.".to_string(),
            Some(span),
            "undefined alias",
        )
    }

    /// A node that must be a mapping is not one (`context` names the site, e.g. "a plugin").
    pub fn expected_mapping(&self, context: &str, span: Span) -> ConfigDiagnostic {
        self.make(
            format!("Expected a mapping for {context}."),
            Some(span),
            "expected a mapping",
        )
    }

    /// An `arguments`/`variables` entry with a non-string value.
    pub fn expected_string(&self, context: &str, span: Span) -> ConfigDiagnostic {
        self.make(
            format!("Expected a string value in {context}."),
            Some(span),
            "expected a string",
        )
    }

    /// `run:` is not `plugin.middleware`.
    pub fn invalid_run(&self, value: &str, span: Span) -> ConfigDiagnostic {
        self.make(
            format!("'{value}' is not a valid run reference; use the format 'plugin.middleware'."),
            Some(span),
            "expected 'plugin.middleware'",
        )
    }

    /// A step missing its required `run:`.
    pub fn missing_run(&self, step: &str, span: Span) -> ConfigDiagnostic {
        self.make(
            format!("Step '{step}' is missing a 'run' entry."),
            Some(span),
            "add a 'run: plugin.middleware' entry",
        )
    }

    /// A `filesystem:` permission value that isn't one of the recognized access levels.
    pub fn invalid_filesystem(&self, value: &str, span: Span) -> ConfigDiagnostic {
        self.make(
            format!(
                "Invalid filesystem access: {value}. Expected one of: none, read-only, read-write."
            ),
            Some(span),
            "unrecognized filesystem access level",
        )
    }

    /// A non-boolean value where a bool is required (engine-chosen; spec mandates shape only).
    pub fn invalid_bool(&self, field: &str, value: &str, span: Span) -> ConfigDiagnostic {
        self.make(
            format!("Invalid {field} value: {value}. Expected 'true' or 'false'."),
            Some(span),
            "expected 'true' or 'false'",
        )
    }

    /// A plugin `url:` that is not an absolute URL with a supported scheme.
    pub fn invalid_url(&self, value: &str, span: Span) -> ConfigDiagnostic {
        self.make(
            format!(
                "Invalid plugin url: {value}. Expected an absolute URL with scheme \
                 'oci', 'file', 'http', or 'https'."
            ),
            Some(span),
            "unsupported or malformed url",
        )
    }

    /// A plugin with no `url:`.
    pub fn missing_url(&self, plugin: &str, span: Span) -> ConfigDiagnostic {
        self.make(
            format!("Plugin '{plugin}' is missing a 'url' entry."),
            Some(span),
            "add a 'url' entry",
        )
    }

    /// No stages found. No span — nothing to point at.
    pub fn no_stages(&self) -> ConfigDiagnostic {
        self.make(
            "No stages defined. A pipeline needs at least one stage.".to_string(),
            None,
            "",
        )
    }

    /// Stages present but no plugins.
    pub fn no_plugins(&self, span: Option<Span>) -> ConfigDiagnostic {
        self.make(
            "No plugins declared. Every step runs a middleware from a plugin, so at least one is required."
                .to_string(),
            span,
            "define at least one plugin",
        )
    }

    /// A `run:` referencing a plugin alias that was not declared (§7.4).
    pub fn plugin_not_found(&self, name: &str, span: Span) -> ConfigDiagnostic {
        self.make(
            format!("No plugin is declared with the alias '{name}'."),
            Some(span),
            "unknown plugin",
        )
    }

    /// A `run:` naming a middleware the plugin does not export (§7.4).
    pub fn middleware_not_found(&self, name: &str, span: Span) -> ConfigDiagnostic {
        self.make(
            format!("The plugin does not export a middleware named '{name}'."),
            Some(span),
            "unknown middleware",
        )
    }

    /// Two plugins declared with the same alias; the second is rejected rather than shadowing the first.
    pub fn duplicate_plugin(&self, name: &str, span: Span) -> ConfigDiagnostic {
        self.make(
            format!("Duplicate plugin name '{name}'. Plugin names must be unique."),
            Some(span),
            "duplicate plugin name",
        )
    }

    /// A key the schema does not define. Silently ignoring these hid typos such as `pluigns:`,
    /// which produced a pipeline with no plugins and no diagnostic at all.
    pub fn unknown_key(&self, key: &str, context: &str, span: Span) -> ConfigDiagnostic {
        self.make(
            format!("Unknown {context} key '{key}'."),
            Some(span),
            "unknown key",
        )
    }

    /// The same schema key given twice. YAML forbids duplicate mapping keys; accepting them and
    /// keeping the last silently discarded whatever the author wrote first.
    pub fn duplicate_key(&self, key: &str, span: Span) -> ConfigDiagnostic {
        self.make(
            format!("Duplicate key '{key}'."),
            Some(span),
            "duplicate key",
        )
    }

    /// A schema key present but with no value. The schema is Moonlit's own contract and must be
    /// precise; a null inside a plugin's arbitrary config map is left alone, because there it may
    /// be a value the plugin acts on.
    pub fn null_value(&self, key: &str, expected: &str, span: Span) -> ConfigDiagnostic {
        self.make(
            format!("Key '{key}' expects {expected}, but has no value."),
            Some(span),
            "missing value",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::Span;

    fn src<'a>() -> Source<'a> {
        Source::new("name: demo\nplugins: []\n", "release.yml")
    }

    #[test]
    fn invalid_run_names_the_bad_reference_and_has_a_span() {
        let d = src().invalid_run("gitpush", Span::new(6, 13));
        assert_eq!(
            d.message(),
            "'gitpush' is not a valid run reference; use the format 'plugin.middleware'."
        );
        // Display (thiserror) matches the message.
        assert_eq!(format!("{d}"), d.message());
        let ss = d.span().expect("has a span");
        assert_eq!(ss.offset(), 6);
        assert_eq!(ss.len(), 7);
    }

    #[test]
    fn no_stages_has_no_span() {
        let d = src().no_stages();
        assert_eq!(
            d.message(),
            "No stages defined. A pipeline needs at least one stage."
        );
        assert!(d.span().is_none());
    }

    #[test]
    fn no_plugins_names_the_requirement_and_has_a_span() {
        let d = src().no_plugins(Some(Span::new(0, 4)));
        assert_eq!(
            d.message(),
            "No plugins declared. Every step runs a middleware from a plugin, so at least one is required."
        );
        assert!(d.span().is_some());
    }

    #[test]
    fn plugin_and_middleware_not_found_name_the_missing_item() {
        assert_eq!(
            src().plugin_not_found("gh", Span::new(0, 2)).message(),
            "No plugin is declared with the alias 'gh'."
        );
        assert_eq!(
            src().middleware_not_found("tag", Span::new(0, 3)).message(),
            "The plugin does not export a middleware named 'tag'."
        );
    }

    #[test]
    fn duplicate_plugin_is_verbatim() {
        assert_eq!(
            src().duplicate_plugin("gh", Span::new(0, 2)).message(),
            "Duplicate plugin name 'gh'. Plugin names must be unique."
        );
    }
}
