//! Single-source plugin resolution (§4.3, §7.3, §8). Turns a plugin URL into a path to verified,
//! ready-to-instantiate WebAssembly component bytes. This module owns scheme parsing and the shared
//! error/option/result types; the per-scheme resolvers live in sibling modules.

use std::path::PathBuf;
use std::time::Duration;

use sha2::{Digest, Sha256};

/// A plugin source, parsed from its URL. The `oci`/`http` variants keep their string form; `file`
/// is normalized to an absolute path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginSource {
    /// OCI reference WITHOUT the `oci://` scheme prefix (`host/namespace/name:tag` or `...@sha256:...`).
    Oci(String),
    /// Absolute local path to a component file.
    File(PathBuf),
    /// Full `http`/`https` URL to a component file.
    Http(String),
}

impl PluginSource {
    /// Parse a plugin URL (§4.3). `nuget://` and unknown schemes produce [`ResolveError::UnsupportedScheme`];
    /// a URL with no scheme produces [`ResolveError::InvalidReference`].
    pub fn parse(url: &str) -> Result<Self, ResolveError> {
        let url = url.trim();
        if let Some(rest) = url.strip_prefix("oci://") {
            return Ok(PluginSource::Oci(rest.to_string()));
        }
        if let Some(rest) = url.strip_prefix("file://") {
            // `file:///abs/path` -> `/abs/path`; `file://relative` stays as-is.
            return Ok(PluginSource::File(PathBuf::from(rest)));
        }
        if url.starts_with("http://") || url.starts_with("https://") {
            return Ok(PluginSource::Http(url.to_string()));
        }
        if let Some(scheme) = url.split_once("://").map(|(s, _)| s) {
            if scheme == "nuget" {
                return Err(ResolveError::UnsupportedScheme {
                    scheme: "nuget".to_string(),
                    hint: "The 'nuget://' plugin scheme was removed in Moonlit 2.0. Publish the plugin to an OCI registry and reference it as 'oci://<host>/<namespace>/<name>:<tag>'.".to_string(),
                });
            }
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

/// Options controlling resolution behavior.
#[derive(Debug, Clone)]
pub struct ResolveOptions {
    /// When true, never touch the network: a cache miss becomes [`ResolveError::OfflineMiss`].
    pub offline: bool,
    /// How long a cached OCI tag→digest resolution stays fresh (§8.3).
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

/// A resolved plugin: a path to verified component bytes plus provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPlugin {
    /// Path to the ready-to-instantiate component bytes on disk.
    pub wasm_path: PathBuf,
    /// The canonical source reference, echoed back.
    pub source: String,
    /// The content digest (`sha256:...`) for OCI sources; `None` for `file`/`http`.
    pub digest: Option<String>,
    /// True when resolution completed without any network request.
    pub cached: bool,
    /// Middleware names declared in the OCI config's `moonlit` block, when present.
    pub middlewares: Option<Vec<String>>,
}

/// A progress callback: `(bytes_received, total_bytes_if_known)`. The host phase adapts this into
/// `PluginPullProgress`; this phase never emits pipeline events.
pub type ProgressFn<'a> = &'a (dyn Fn(u64, Option<u64>) + Send + Sync);

/// Errors from resolution. Maps to exit code 3 (`PluginLoad`) at the future `EngineError` layer.
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

/// Lowercase hex SHA-256 of a string. Used for content-addressing cache keys.
// Not yet called outside tests: `cache`/`http`/`oci` (later Phase 4 tasks) will call this.
#[allow(dead_code)]
pub(crate) fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_oci_scheme_stripping_prefix() {
        let s = PluginSource::parse("oci://registry.moonlitbuild.dev/wolfware/git:2.0.0").unwrap();
        assert_eq!(
            s,
            PluginSource::Oci("registry.moonlitbuild.dev/wolfware/git:2.0.0".to_string())
        );
    }

    #[test]
    fn parses_file_scheme_to_absolute_path() {
        let s = PluginSource::parse("file:///abs/path/plugin.wasm").unwrap();
        assert_eq!(
            s,
            PluginSource::File(PathBuf::from("/abs/path/plugin.wasm"))
        );
    }

    #[test]
    fn parses_http_and_https_keeping_full_url() {
        assert_eq!(
            PluginSource::parse("https://example.com/p.wasm").unwrap(),
            PluginSource::Http("https://example.com/p.wasm".to_string())
        );
        assert_eq!(
            PluginSource::parse("http://example.com/p.wasm").unwrap(),
            PluginSource::Http("http://example.com/p.wasm".to_string())
        );
    }

    #[test]
    fn nuget_scheme_is_a_hard_error_with_migration_hint() {
        let err = PluginSource::parse("nuget://Some.Plugin/1.0.0").unwrap_err();
        match err {
            ResolveError::UnsupportedScheme { scheme, hint } => {
                assert_eq!(scheme, "nuget");
                assert_eq!(
                    hint,
                    "The 'nuget://' plugin scheme was removed in Moonlit 2.0. Publish the plugin to an OCI registry and reference it as 'oci://<host>/<namespace>/<name>:<tag>'."
                );
            }
            other => panic!("expected UnsupportedScheme, got {other:?}"),
        }
    }

    #[test]
    fn unknown_scheme_is_unsupported_without_nuget_hint() {
        let err = PluginSource::parse("ftp://example.com/p.wasm").unwrap_err();
        match err {
            ResolveError::UnsupportedScheme { scheme, hint } => {
                assert_eq!(scheme, "ftp");
                assert_eq!(hint, "Supported schemes are: oci, file, http, https.");
            }
            other => panic!("expected UnsupportedScheme, got {other:?}"),
        }
    }

    #[test]
    fn missing_scheme_is_invalid_reference() {
        let err = PluginSource::parse("registry.example.com/git:1.0.0").unwrap_err();
        assert!(matches!(err, ResolveError::InvalidReference(_)));
    }

    #[test]
    fn default_options_are_online_with_15_minute_ttl() {
        let o = ResolveOptions::default();
        assert!(!o.offline);
        assert_eq!(o.tag_ttl, std::time::Duration::from_secs(15 * 60));
    }

    #[test]
    fn sha256_hex_is_stable_and_lowercase() {
        assert_eq!(
            sha256_hex("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
