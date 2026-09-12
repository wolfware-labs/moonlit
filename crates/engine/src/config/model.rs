//! Typed configuration model for a Moonlit pipeline.
//!
//! Produced by [`crate::config::parse_config`]. Scalars stay raw strings; `$(...)` text is
//! preserved verbatim and never resolved here (that is Phase 3).

use indexmap::IndexMap;

/// A byte range into the source YAML.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// A zero-length span at a single offset.
    pub fn point(at: usize) -> Self {
        Self { start: at, end: at }
    }

    pub fn to_source_span(self) -> miette::SourceSpan {
        (self.start, self.end.saturating_sub(self.start)).into()
    }
}

/// A value paired with its source span.
#[derive(Clone, Debug, PartialEq)]
pub struct Spanned<T> {
    pub value: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(value: T, span: Span) -> Self {
        Self { value, span }
    }
}

/// An order-preserving map of config keys to spanned values.
pub type ConfigMap = IndexMap<String, Spanned<ConfigValue>>;

/// A dynamic configuration value. Scalars are kept as raw strings until Phase 3 binding.
#[derive(Clone, Debug, PartialEq)]
pub enum ConfigValue {
    Null,
    String(String),
    List(Vec<Spanned<ConfigValue>>),
    Map(ConfigMap),
}

/// A fully parsed, validated pipeline configuration.
#[derive(Clone, Debug, PartialEq)]
pub struct PipelineConfig {
    pub name: String,
    pub arguments: IndexMap<String, String>,
    pub variables: IndexMap<String, String>,
    pub plugins: Spanned<Vec<Plugin>>,
    pub stages: Spanned<Vec<Stage>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stage {
    pub name: String,
    pub steps: Vec<Step>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Plugin {
    pub name: String,
    pub url: Spanned<PluginUrl>,
    pub config: ConfigMap,
    pub permissions: Option<Permissions>,
}

/// A plugin source URL, classified by scheme. Holds the original URL string; structural
/// resolution (host/path/existence) happens in `resolve/` (M2+).
#[derive(Clone, Debug, PartialEq)]
pub enum PluginUrl {
    Oci(String),
    File(String),
    Http(String),
    Https(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    pub name: String,
    pub run: Spanned<Run>,
    pub condition: Option<String>,
    pub halt_if: Option<String>,
    pub continue_on_error: bool,
    pub config: ConfigMap,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    pub plugin: String,
    pub middleware: String,
}

/// Per-plugin sandboxing grants (§3.3). Omitted in YAML → [`Permissions::deny`].
#[derive(Clone, Debug, PartialEq)]
pub struct Permissions {
    pub network: Vec<String>,
    pub exec: Vec<String>,
    pub env: Vec<String>,
    pub filesystem: FilesystemAccess,
}

impl Permissions {
    /// An explicit full-trust grant, still used by tests. Not the default (§3.3).
    pub fn full_trust() -> Self {
        Self {
            network: vec!["*".to_string()],
            exec: vec!["*".to_string()],
            env: vec!["*".to_string()],
            filesystem: FilesystemAccess::ReadWrite,
        }
    }

    /// Deny-by-default grant: the plugin gets no host access unless its `permissions`
    /// block names it (§3.3). Omitted `permissions`, and any key a present block does
    /// not name, resolve to these.
    pub fn deny() -> Self {
        Self {
            network: vec![],
            exec: vec![],
            env: vec![],
            filesystem: FilesystemAccess::None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilesystemAccess {
    None,
    ReadOnly,
    ReadWrite,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permissions_full_trust_defaults() {
        let p = Permissions::full_trust();
        assert_eq!(p.network, vec!["*".to_string()]);
        assert_eq!(p.exec, vec!["*".to_string()]);
        assert_eq!(p.env, vec!["*".to_string()]);
        assert_eq!(p.filesystem, FilesystemAccess::ReadWrite);
    }

    #[test]
    fn permissions_deny_grants_nothing() {
        let p = Permissions::deny();
        assert!(p.network.is_empty());
        assert!(p.exec.is_empty());
        assert!(p.env.is_empty());
        assert_eq!(p.filesystem, FilesystemAccess::None);
    }

    #[test]
    fn span_to_source_span_is_offset_and_len() {
        let ss = Span::new(3, 10).to_source_span();
        assert_eq!(ss.offset(), 3);
        assert_eq!(ss.len(), 7);
    }

    #[test]
    fn span_point_is_zero_length() {
        let ss = Span::point(5).to_source_span();
        assert_eq!(ss.offset(), 5);
        assert_eq!(ss.len(), 0);
    }
}
