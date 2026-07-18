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
    /// The human-readable message (parity strings live in the [`Source`] constructors).
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

    /// `run:` is not `plugin.middleware` (verbatim parity string).
    pub fn invalid_run(&self, value: &str, span: Span) -> ConfigDiagnostic {
        self.make(
            format!("Invalid run format: {value}. Expected format: 'plugin.middleware'"),
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

    /// A plugin `url:` using the removed `nuget://` scheme.
    pub fn nuget_removed(&self, span: Span) -> ConfigDiagnostic {
        self.make(
            "The 'nuget://' plugin URL scheme was removed in Moonlit 2.0. \
             Publish the plugin to an OCI registry and use 'oci://' instead."
                .to_string(),
            Some(span),
            "migrate to 'oci://'",
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

    /// No stages found (verbatim parity string). No span — nothing to point at.
    pub fn no_stages(&self) -> ConfigDiagnostic {
        self.make(
            "No stages found in the release configuration.".to_string(),
            None,
            "",
        )
    }

    /// Stages present but no plugins (verbatim parity string).
    pub fn no_plugins(&self, span: Option<Span>) -> ConfigDiagnostic {
        self.make(
            "At least one plugin configuration must be provided.".to_string(),
            span,
            "define at least one plugin",
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
    fn invalid_run_uses_verbatim_message_and_span() {
        let d = src().invalid_run("gitpush", Span::new(6, 13));
        assert_eq!(
            d.message(),
            "Invalid run format: gitpush. Expected format: 'plugin.middleware'"
        );
        // Display (thiserror) matches the message.
        assert_eq!(format!("{d}"), d.message());
        let ss = d.span().expect("has a span");
        assert_eq!(ss.offset(), 6);
        assert_eq!(ss.len(), 7);
    }

    #[test]
    fn no_stages_is_verbatim_and_spanless() {
        let d = src().no_stages();
        assert_eq!(d.message(), "No stages found in the release configuration.");
        assert!(d.span().is_none());
    }

    #[test]
    fn no_plugins_is_verbatim() {
        let d = src().no_plugins(Some(Span::new(0, 4)));
        assert_eq!(
            d.message(),
            "At least one plugin configuration must be provided."
        );
        assert!(d.span().is_some());
    }

    #[test]
    fn nuget_is_rejected_with_migration_hint() {
        let d = src().nuget_removed(Span::new(0, 5));
        assert!(d.message().contains("nuget://"));
        assert!(d.message().contains("oci://"));
    }
}
