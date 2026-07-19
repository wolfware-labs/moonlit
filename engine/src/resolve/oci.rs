//! `oci://` resolution (§8). This file holds the [`RegistryClient`] seam over `oci-client` (so the
//! orchestration in `resolve_oci` is unit-tested against a mock) plus artifact verification helpers.
#![allow(dead_code)] // TODO(Task 8): remove once the public resolve() dispatcher wires the OCI resolver.

use oci_client::manifest::{OciDescriptor, OciImageManifest};
use oci_client::secrets::RegistryAuth;
use oci_client::{Client, Reference};

use crate::cache::{Cache, PluginMeta};
use crate::resolve::{ProgressFn, ResolveError, ResolveOptions, ResolvedPlugin};

/// The OCI config media type Moonlit plugin artifacts use (§8.1).
pub(crate) const CONFIG_MEDIA_TYPE: &str = "application/vnd.wasm.config.v0+json";
/// Accepted layer media types (§8.1); the CNCF wasm-OCI convention plus the older deislabs constant.
pub(crate) const LAYER_MEDIA_TYPES: [&str; 2] = [
    "application/wasm",
    "application/vnd.wasm.content.layer.v1+wasm",
];

/// A narrow seam over the OCI registry, so `resolve_oci` can be tested without a network.
pub(crate) trait RegistryClient {
    /// Pull the (single-platform) image manifest and its digest.
    async fn pull_image_manifest(
        &self,
        reference: &Reference,
        auth: &RegistryAuth,
    ) -> Result<(OciImageManifest, String), ResolveError>;

    /// Pull a blob (config or layer) identified by its descriptor. The implementation verifies the
    /// content digest.
    async fn pull_blob(
        &self,
        reference: &Reference,
        descriptor: &OciDescriptor,
    ) -> Result<Vec<u8>, ResolveError>;
}

/// The real registry client, backed by `oci-client`.
pub(crate) struct OciClient(Client);

/// Construct the real client with default configuration (rustls transport).
pub(crate) fn new_client() -> OciClient {
    OciClient(Client::default())
}

impl RegistryClient for OciClient {
    async fn pull_image_manifest(
        &self,
        reference: &Reference,
        auth: &RegistryAuth,
    ) -> Result<(OciImageManifest, String), ResolveError> {
        self.0
            .pull_image_manifest(reference, auth)
            .await
            .map_err(map_oci_error)
    }

    async fn pull_blob(
        &self,
        reference: &Reference,
        descriptor: &OciDescriptor,
    ) -> Result<Vec<u8>, ResolveError> {
        let mut buf: Vec<u8> = Vec::with_capacity(descriptor.size.max(0) as usize);
        self.0
            .pull_blob(reference, descriptor, &mut buf)
            .await
            .map_err(map_oci_error)?;
        Ok(buf)
    }
}

/// Map an `oci-client` error to a [`ResolveError`], classifying auth/not-found/digest cases.
fn map_oci_error(err: oci_client::errors::OciDistributionError) -> ResolveError {
    let msg = err.to_string();
    let lower = msg.to_lowercase();
    if lower.contains("unauthorized")
        || lower.contains("authentication")
        || lower.contains("401")
        || lower.contains("403")
    {
        ResolveError::Auth(msg)
    } else if lower.contains("not found") || lower.contains("404") {
        ResolveError::NotFound(msg)
    } else if lower.contains("digest") && lower.contains("mismatch") {
        ResolveError::DigestMismatch(msg)
    } else {
        ResolveError::Network(msg)
    }
}

/// Verify the artifact is a Moonlit/wasm plugin by its config + layer media types (§8.1).
pub(crate) fn verify_media_types(
    config_media_type: &str,
    layer_media_type: &str,
) -> Result<(), ResolveError> {
    if config_media_type != CONFIG_MEDIA_TYPE {
        return Err(ResolveError::MediaTypeMismatch(format!(
            "config media type is '{config_media_type}', expected '{CONFIG_MEDIA_TYPE}'"
        )));
    }
    if !LAYER_MEDIA_TYPES.contains(&layer_media_type) {
        return Err(ResolveError::MediaTypeMismatch(format!(
            "layer media type is '{layer_media_type}', expected one of {LAYER_MEDIA_TYPES:?}"
        )));
    }
    Ok(())
}

/// Extract `moonlit.middlewares` from the OCI config JSON (§8.1), if present.
pub(crate) fn parse_middlewares(config_json: &[u8]) -> Option<Vec<String>> {
    let value: serde_json::Value = serde_json::from_slice(config_json).ok()?;
    let arr = value.get("moonlit")?.get("middlewares")?.as_array()?;
    Some(
        arr.iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
    )
}

/// Resolve an `oci://` source to a cached component path (§8.3). Digest resolution: a pinned digest
/// is used directly; a tag consults the `refs/` TTL cache, else pulls the manifest. A `plugins/<digest>`
/// hit short-circuits with no blob fetch. `cached` is true only when no network request was made.
pub(crate) async fn resolve_oci<C: RegistryClient>(
    raw_ref: &str,
    opts: &ResolveOptions,
    cache: &Cache,
    client: &C,
    auth: RegistryAuth,
    progress: Option<ProgressFn<'_>>,
) -> Result<ResolvedPlugin, ResolveError> {
    let reference: Reference = raw_ref
        .parse()
        .map_err(|e| ResolveError::InvalidReference(format!("'{raw_ref}': {e}")))?;
    let source = format!("oci://{raw_ref}");

    let mut network = false;
    let mut fetched: Option<(OciImageManifest, String)> = None;

    // Step A — determine the manifest digest.
    let manifest_digest: String = if let Some(pinned) = reference.digest() {
        pinned.to_string()
    } else if let Some(cached_digest) = cache.read_ref(raw_ref, opts.tag_ttl) {
        cached_digest
    } else if opts.offline {
        return Err(ResolveError::OfflineMiss(source));
    } else {
        let (manifest, digest) = client.pull_image_manifest(&reference, &auth).await?;
        network = true;
        cache
            .write_ref(raw_ref, &digest)
            .map_err(|e| ResolveError::Io(e.to_string()))?;
        fetched = Some((manifest, digest.clone()));
        digest
    };

    // Filesystem-safe cache key: a digest is `algo:hex`, and ':' is illegal in a Windows path
    // component, so map it to `algo-hex` for the `plugins/<key>` directory. The canonical
    // `algo:hex` digest is preserved in `ResolvedPlugin.digest` and `meta.json`.
    let cache_key = manifest_digest.replace(':', "-");

    // Step B — plugin cache hit?
    if cache.has_plugin(&cache_key) {
        let middlewares = cache.read_meta(&cache_key).and_then(|m| m.middlewares);
        return Ok(ResolvedPlugin {
            wasm_path: cache.plugin_wasm(&cache_key),
            source,
            digest: Some(manifest_digest),
            cached: !network,
            middlewares,
        });
    }

    // Step C — need the manifest (and thus the network) to pull the blob. Past the Step B cache hit,
    // resolution is necessarily a fresh pull, so `cached` is hard-coded false below and `network`
    // is not tracked further.
    if opts.offline {
        return Err(ResolveError::OfflineMiss(source));
    }
    let (manifest, _digest) = match fetched {
        Some(pair) => pair,
        None => client.pull_image_manifest(&reference, &auth).await?,
    };

    // Step D — verify it is a wasm/Moonlit artifact.
    let layer = manifest
        .layers
        .first()
        .ok_or_else(|| ResolveError::MediaTypeMismatch("artifact has no layers".to_string()))?;
    verify_media_types(&manifest.config.media_type, &layer.media_type)?;

    // Step E — middlewares from the config blob.
    let config_bytes = client.pull_blob(&reference, &manifest.config).await?;
    let middlewares = parse_middlewares(&config_bytes);

    // Step F — pull the layer, store in the blob store and plugin store.
    let bytes = client.pull_blob(&reference, layer).await?;
    if let Some(report) = progress {
        report(bytes.len() as u64, Some(bytes.len() as u64));
    }
    cache
        .write_blob(&layer.digest, &bytes)
        .map_err(|e| ResolveError::Io(e.to_string()))?;
    let meta = PluginMeta {
        source: source.clone(),
        digest: Some(manifest_digest.clone()),
        layer_digest: Some(layer.digest.clone()),
        size: bytes.len() as u64,
        pulled_at: cache.now_unix(),
        middlewares: middlewares.clone(),
    };
    let wasm_path = cache
        .store_plugin(&cache_key, &meta, &bytes)
        .map_err(|e| ResolveError::Io(e.to_string()))?;

    Ok(ResolvedPlugin {
        wasm_path,
        source,
        digest: Some(manifest_digest),
        cached: false,
        middlewares,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_documented_wasm_media_types() {
        assert!(
            verify_media_types("application/vnd.wasm.config.v0+json", "application/wasm").is_ok()
        );
        assert!(
            verify_media_types(
                "application/vnd.wasm.config.v0+json",
                "application/vnd.wasm.content.layer.v1+wasm"
            )
            .is_ok()
        );
    }

    #[test]
    fn rejects_wrong_config_media_type() {
        let err = verify_media_types(
            "application/vnd.oci.image.config.v1+json",
            "application/wasm",
        )
        .unwrap_err();
        assert!(matches!(err, ResolveError::MediaTypeMismatch(_)));
    }

    #[test]
    fn rejects_wrong_layer_media_type() {
        let err = verify_media_types(
            "application/vnd.wasm.config.v0+json",
            "application/octet-stream",
        )
        .unwrap_err();
        assert!(matches!(err, ResolveError::MediaTypeMismatch(_)));
    }

    #[test]
    fn parses_middlewares_from_moonlit_block() {
        let json =
            br#"{"moonlit":{"world":"moonlit:plugin@2.0.0","middlewares":["build","test"]}}"#;
        assert_eq!(
            parse_middlewares(json),
            Some(vec!["build".to_string(), "test".to_string()])
        );
    }

    #[test]
    fn missing_moonlit_block_yields_none() {
        assert_eq!(parse_middlewares(br#"{"layerDigests":[]}"#), None);
    }

    #[test]
    fn malformed_config_json_yields_none() {
        assert_eq!(parse_middlewares(b"not json"), None);
    }

    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::cache::Clock;

    struct ZeroClock;
    impl Clock for ZeroClock {
        fn now_unix(&self) -> u64 {
            0
        }
    }

    fn cache() -> (Cache, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        (
            Cache::with_root_and_clock(dir.path().to_path_buf(), Box::new(ZeroClock)),
            dir,
        )
    }

    fn wasm_manifest(
        config_json: &'static [u8],
        layer_bytes: &'static [u8],
    ) -> (OciImageManifest, Vec<u8>) {
        let manifest = OciImageManifest {
            schema_version: 2,
            config: OciDescriptor {
                media_type: CONFIG_MEDIA_TYPE.to_string(),
                digest: "sha256:config".to_string(),
                size: config_json.len() as i64,
                ..Default::default()
            },
            layers: vec![OciDescriptor {
                media_type: "application/wasm".to_string(),
                digest: "sha256:layer".to_string(),
                size: layer_bytes.len() as i64,
                ..Default::default()
            }],
            ..Default::default()
        };
        (manifest, layer_bytes.to_vec())
    }

    /// A mock registry client with call counters and canned data.
    struct MockClient {
        manifest: OciImageManifest,
        manifest_digest: String,
        config_json: Vec<u8>,
        layer_bytes: Vec<u8>,
        manifest_calls: Arc<AtomicUsize>,
        blob_calls: Arc<AtomicUsize>,
    }

    impl RegistryClient for MockClient {
        async fn pull_image_manifest(
            &self,
            _reference: &Reference,
            _auth: &RegistryAuth,
        ) -> Result<(OciImageManifest, String), ResolveError> {
            self.manifest_calls.fetch_add(1, Ordering::SeqCst);
            Ok((self.manifest.clone(), self.manifest_digest.clone()))
        }

        async fn pull_blob(
            &self,
            _reference: &Reference,
            descriptor: &OciDescriptor,
        ) -> Result<Vec<u8>, ResolveError> {
            self.blob_calls.fetch_add(1, Ordering::SeqCst);
            if descriptor.media_type == CONFIG_MEDIA_TYPE {
                Ok(self.config_json.clone())
            } else {
                Ok(self.layer_bytes.clone())
            }
        }
    }

    fn mock(config_json: &'static [u8], layer_bytes: &'static [u8]) -> MockClient {
        let (manifest, layer) = wasm_manifest(config_json, layer_bytes);
        MockClient {
            manifest,
            manifest_digest: "sha256:manifest".to_string(),
            config_json: config_json.to_vec(),
            layer_bytes: layer,
            manifest_calls: Arc::new(AtomicUsize::new(0)),
            blob_calls: Arc::new(AtomicUsize::new(0)),
        }
    }

    #[tokio::test]
    async fn tag_miss_pulls_stores_and_records_ref_and_middlewares() {
        let (cache, _d) = cache();
        let client = mock(br#"{"moonlit":{"middlewares":["build"]}}"#, b"\0asm-body");
        let resolved = resolve_oci(
            "reg.example.com/wolfware/git:2.0.0",
            &ResolveOptions::default(),
            &cache,
            &client,
            RegistryAuth::Anonymous,
            None,
        )
        .await
        .unwrap();

        assert_eq!(resolved.digest.as_deref(), Some("sha256:manifest"));
        assert!(!resolved.cached);
        assert_eq!(resolved.middlewares, Some(vec!["build".to_string()]));
        assert_eq!(std::fs::read(&resolved.wasm_path).unwrap(), b"\0asm-body");
        // manifest fetched once; config + layer blobs pulled.
        assert_eq!(client.manifest_calls.load(Ordering::SeqCst), 1);
        assert_eq!(client.blob_calls.load(Ordering::SeqCst), 2);
        // the tag→digest ref was recorded.
        assert_eq!(
            cache
                .read_ref(
                    "reg.example.com/wolfware/git:2.0.0",
                    std::time::Duration::from_secs(900)
                )
                .as_deref(),
            Some("sha256:manifest")
        );
    }

    #[tokio::test]
    async fn tag_hit_within_ttl_and_cached_plugin_skips_network() {
        let (cache, _d) = cache();
        let opts = ResolveOptions::default();
        let client = mock(br#"{"moonlit":{"middlewares":["build"]}}"#, b"\0asm-body");

        // First call populates cache + ref.
        resolve_oci(
            "reg/x:1",
            &opts,
            &cache,
            &client,
            RegistryAuth::Anonymous,
            None,
        )
        .await
        .unwrap();
        let before = client.manifest_calls.load(Ordering::SeqCst);

        // Second call: ref fresh + plugin cached -> no more network.
        let second = resolve_oci(
            "reg/x:1",
            &opts,
            &cache,
            &client,
            RegistryAuth::Anonymous,
            None,
        )
        .await
        .unwrap();
        assert!(second.cached);
        assert_eq!(second.middlewares, Some(vec!["build".to_string()]));
        assert_eq!(client.manifest_calls.load(Ordering::SeqCst), before);
    }

    #[tokio::test]
    async fn digest_pinned_hit_skips_network() {
        let (cache, _d) = cache();
        let opts = ResolveOptions::default();
        let client = mock(br#"{"moonlit":{"middlewares":[]}}"#, b"\0asm");
        // A digest-pinned reference requires a valid 64-hex sha256 (oci-client validates the format).
        const DIGEST: &str =
            "sha256:1111111111111111111111111111111111111111111111111111111111111111";
        let pinned_ref = "reg.example.com/x/y@sha256:1111111111111111111111111111111111111111111111111111111111111111";
        // Pre-populate the plugin under the filesystem-safe cache key (':' -> '-').
        let cache_key = DIGEST.replace(':', "-");
        let meta = crate::cache::PluginMeta {
            source: format!("oci://{pinned_ref}"),
            digest: Some(DIGEST.to_string()),
            layer_digest: Some("sha256:layer".to_string()),
            size: 4,
            pulled_at: 0,
            middlewares: Some(vec![]),
        };
        cache.store_plugin(&cache_key, &meta, b"\0asm").unwrap();

        let resolved = resolve_oci(
            pinned_ref,
            &opts,
            &cache,
            &client,
            RegistryAuth::Anonymous,
            None,
        )
        .await
        .unwrap();
        assert!(resolved.cached);
        assert_eq!(resolved.digest.as_deref(), Some(DIGEST));
        assert_eq!(client.manifest_calls.load(Ordering::SeqCst), 0);
        assert_eq!(client.blob_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn offline_with_no_cache_errors() {
        let (cache, _d) = cache();
        let opts = ResolveOptions {
            offline: true,
            ..Default::default()
        };
        let client = mock(br#"{}"#, b"\0asm");
        let err = resolve_oci(
            "reg/x:1",
            &opts,
            &cache,
            &client,
            RegistryAuth::Anonymous,
            None,
        )
        .await
        .unwrap_err();
        assert!(matches!(err, ResolveError::OfflineMiss(_)));
        assert_eq!(client.manifest_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn wrong_media_type_is_rejected() {
        let (cache, _d) = cache();
        let mut client = mock(br#"{}"#, b"\0asm");
        // Corrupt the layer media type.
        client.manifest.layers[0].media_type = "application/octet-stream".to_string();
        let err = resolve_oci(
            "reg/x:1",
            &ResolveOptions::default(),
            &cache,
            &client,
            RegistryAuth::Anonymous,
            None,
        )
        .await
        .unwrap_err();
        assert!(matches!(err, ResolveError::MediaTypeMismatch(_)));
    }
}
