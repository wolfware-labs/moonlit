use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginSource {
    Oci(String),
    File(PathBuf),
    Http(String),
}

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum ResolveError {
    #[error("unsupported plugin URL scheme: '{scheme}'")]
    #[diagnostic(code(moonlit::resolve::unsupported_scheme), help("{hint}"))]
    UnsupportedScheme { scheme: String, hint: String },

    #[error("invalid plugin reference: {0}")]
    #[diagnostic(code(moonlit::resolve::invalid_reference))]
    InvalidReference(String),

    #[error("plugin not found: {0}")]
    #[diagnostic(code(moonlit::resolve::not_found))]
    NotFound(String),

    #[error("not a Moonlit/wasm plugin artifact: {0}")]
    #[diagnostic(code(moonlit::resolve::media_type_mismatch))]
    MediaTypeMismatch(String),

    #[error("plugin content digest mismatch: {0}")]
    #[diagnostic(code(moonlit::resolve::digest_mismatch))]
    DigestMismatch(String),

    #[error("authentication failed for registry: {0}")]
    #[diagnostic(code(moonlit::resolve::auth))]
    Auth(String),

    #[error("network error while resolving plugin: {0}")]
    #[diagnostic(code(moonlit::resolve::network))]
    Network(String),

    #[error("offline: no cached plugin for {0}")]
    #[diagnostic(code(moonlit::resolve::offline_miss))]
    OfflineMiss(String),

    #[error("cache I/O error: {0}")]
    #[diagnostic(code(moonlit::resolve::io))]
    Io(String),
}

impl FromStr for PluginSource {
    type Err = ResolveError;

    fn from_str(url: &str) -> Result<Self, Self::Err> {
        let url = url.trim();
        if let Some(rest) = url.strip_prefix("oci://") {
            return Ok(PluginSource::Oci(rest.to_string()));
        }
        if let Some(rest) = url.strip_prefix("file://") {
            return Ok(PluginSource::File(PathBuf::from(rest)));
        }
        if url.starts_with("http://") || url.starts_with("https://") {
            return Ok(PluginSource::Http(url.to_string()));
        }
        if let Some(scheme) = url.split_once("://").map(|(s, _)| s) {
            return Err(ResolveError::UnsupportedScheme {
                scheme: scheme.to_string(),
                hint: "Supported schemes are: oci, file, http, https.".to_string(),
            });
        }
        Err(ResolveError::InvalidReference(format!(
            "'{url}' has no URL scheme; expected one of oci://, file://, http://, https://"
        )))
    }
}

#[derive(Debug, Clone)]
pub struct ResolveOptions {
    pub offline: bool,
    pub tag_ttl: Duration,
}

impl Default for ResolveOptions {
    fn default() -> Self {
        Self {
            offline: false,
            tag_ttl: Duration::from_secs(15 * 60),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPlugin {
    pub wasm_path: PathBuf,
    pub source: String,
    pub digest: Option<String>,
    pub cached: bool,
    pub middlewares: Option<Vec<String>>,
}

pub type ProgressFn<'a> = &'a (dyn Fn(u64, Option<u64>) + Send + Sync);
