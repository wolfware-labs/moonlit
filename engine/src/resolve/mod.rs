//! Single-source plugin resolution (§4.3, §7.3, §8). Turns a plugin URL into a path to verified,
//! ready-to-instantiate WebAssembly component bytes. This module owns scheme parsing and the shared
//! error/option/result types; the per-scheme resolvers live in sibling modules.

pub(crate) mod auth;
pub(crate) mod file;
pub(crate) mod http;
pub(crate) mod oci;

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
pub(crate) fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

use crate::cache::Cache;

/// Resolve ONE plugin source to verified component bytes on disk (§8). Dispatches by scheme; OCI
/// credentials are read from the user's home config. This never instantiates a component and never
/// emits pipeline events — the host/pipeline phase adapts the [`ProgressFn`] into `PluginPullProgress`.
pub async fn resolve(
    source: &PluginSource,
    opts: &ResolveOptions,
    cache: &Cache,
    progress: Option<ProgressFn<'_>>,
) -> Result<ResolvedPlugin, ResolveError> {
    match source {
        PluginSource::File(path) => file::resolve_file(path),
        PluginSource::Http(url) => http::resolve_http(url, opts, cache, progress).await,
        PluginSource::Oci(raw_ref) => {
            let reference: oci_client::Reference = raw_ref
                .parse()
                .map_err(|e| ResolveError::InvalidReference(format!("'{raw_ref}': {e}")))?;
            let home = dirs::home_dir().unwrap_or_default();
            let auth = auth::resolve_auth(reference.registry(), &home);
            let client = oci::new_client();
            oci::resolve_oci(raw_ref, opts, cache, &client, auth, progress).await
        }
    }
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

    // dispatcher tests — added to the existing tests module
    use crate::cache::{Cache, Clock};

    struct ZeroClock;
    impl Clock for ZeroClock {
        fn now_unix(&self) -> u64 {
            0
        }
    }
    fn tmp_cache() -> (Cache, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        (
            Cache::with_root_and_clock(dir.path().to_path_buf(), Box::new(ZeroClock)),
            dir,
        )
    }

    #[tokio::test]
    async fn dispatches_file_source() {
        let dir = tempfile::tempdir().unwrap();
        let wasm = dir.path().join("p.wasm");
        std::fs::write(&wasm, b"\0asm").unwrap();
        let (cache, _c) = tmp_cache();
        let source = PluginSource::File(wasm.clone());
        let resolved = resolve(&source, &ResolveOptions::default(), &cache, None)
            .await
            .unwrap();
        assert_eq!(resolved.wasm_path, wasm);
        assert!(resolved.cached);
    }

    #[tokio::test]
    async fn dispatches_http_source_offline_miss() {
        let (cache, _c) = tmp_cache();
        let source = PluginSource::Http("http://127.0.0.1:1/p.wasm".to_string());
        let opts = ResolveOptions {
            offline: true,
            ..Default::default()
        };
        let err = resolve(&source, &opts, &cache, None).await.unwrap_err();
        assert!(matches!(err, ResolveError::OfflineMiss(_)));
    }

    #[tokio::test]
    async fn dispatches_oci_source_offline_miss() {
        let (cache, _c) = tmp_cache();
        let source = PluginSource::Oci("reg.example.com/x/y:1".to_string());
        let opts = ResolveOptions {
            offline: true,
            ..Default::default()
        };
        let err = resolve(&source, &opts, &cache, None).await.unwrap_err();
        assert!(matches!(err, ResolveError::OfflineMiss(_)));
    }

    /// Live smoke test against a real public wasm OCI artifact. Ignored by default (network + TLS).
    /// Run manually with: `cargo test -p moonlit-engine live_oci -- --ignored --nocapture`.
    #[tokio::test]
    #[ignore = "network: pulls a real public OCI wasm artifact"]
    async fn live_oci_pull_smoke() {
        let (cache, _c) = tmp_cache();
        // A small, public wasm component artifact. Replace with a Moonlit-published plugin once one exists.
        let source = PluginSource::Oci("ghcr.io/webassembly/wasi/hello-world:latest".to_string());
        let resolved = resolve(&source, &ResolveOptions::default(), &cache, None)
            .await
            .unwrap();
        assert!(resolved.wasm_path.is_file());
        assert!(resolved.digest.is_some());
    }
}
