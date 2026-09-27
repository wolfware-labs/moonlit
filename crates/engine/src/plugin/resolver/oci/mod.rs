pub mod auth;
mod client;

use crate::cache::{Cache, PluginMeta};
use crate::plugin::resolver::oci::client::OciClient;
use crate::plugin::resolver::{ProgressFn, ResolveError, ResolveOptions, ResolvedPlugin};
use oci_client::Reference;
use oci_client::manifest::OciImageManifest;
use std::path::Path;

pub const CONFIG_MEDIA_TYPE: &str = "application/vnd.wasm.config.v0+json";
const LAYER_MEDIA_TYPES: [&str; 2] = ["application/wasm", "application/vnd.wasm.content.layer.v1+wasm"];

pub async fn resolve_oci(
  raw_ref: &str,
  opts: &ResolveOptions,
  cache: &Cache,
  progress: Option<ProgressFn<'_>>,
) -> Result<ResolvedPlugin, ResolveError> {
  let home = crate::paths::home_dir().unwrap_or_default();
  resolve_with(&OciClient::new(), &home, raw_ref, opts, cache, progress).await
}

async fn resolve_with(
  client: &OciClient,
  home: &Path,
  raw_ref: &str,
  opts: &ResolveOptions,
  cache: &Cache,
  progress: Option<ProgressFn<'_>>,
) -> Result<ResolvedPlugin, ResolveError> {
  let reference: Reference = raw_ref
    .parse()
    .map_err(|e| ResolveError::InvalidReference(format!("'{raw_ref}': {e}")))?;
  let auth = auth::resolve_auth(reference.registry(), home);
  let source = format!("oci://{raw_ref}");

  let mut network = false;
  let mut fetched: Option<(OciImageManifest, String)> = None;

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

  let cache_key = manifest_digest.replace(':', "-");

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

  if opts.offline {
    return Err(ResolveError::OfflineMiss(source));
  }

  let (manifest, fetched_digest) = match fetched {
    Some(pair) => pair,
    None => client.pull_image_manifest(&reference, &auth).await?,
  };
  let manifest_digest = fetched_digest;
  let cache_key = manifest_digest.replace(':', "-");

  let layer = manifest
    .layers
    .first()
    .ok_or_else(|| ResolveError::MediaTypeMismatch("artifact has no layers".to_string()))?;
  verify_media_types(&manifest.config.media_type, &layer.media_type)?;

  let config_bytes = client.pull_blob(&reference, &manifest.config).await?;
  let middlewares = parse_middlewares(&config_bytes);

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

fn verify_media_types(config_media_type: &str, layer_media_type: &str) -> Result<(), ResolveError> {
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

#[must_use]
fn parse_middlewares(config_json: &[u8]) -> Option<Vec<String>> {
  let value: serde_json::Value = serde_json::from_slice(config_json).ok()?;
  let arr = value.get("moonlit")?.get("middlewares")?.as_array()?;
  Some(arr.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::cache::Clock;
  use sha2::{Digest, Sha256};
  use std::sync::Arc;
  use std::sync::Mutex;
  use std::sync::atomic::{AtomicU64, Ordering};
  use std::time::Duration;
  use wiremock::matchers::{method, path};
  use wiremock::{Mock, MockServer, ResponseTemplate};

  const WASM: &[u8] = b"\0asm-oci-plugin";
  const MANIFEST_MEDIA_TYPE: &str = "application/vnd.oci.image.manifest.v1+json";

  struct ManualClock(Arc<AtomicU64>);

  impl Clock for ManualClock {
    fn now_unix(&self) -> u64 {
      self.0.load(Ordering::SeqCst)
    }
  }

  struct Registry {
    _server: MockServer,
    host: String,
    manifest_digest: String,
  }

  struct Artifact {
    config_media_type: &'static str,
    layer_media_type: &'static str,
    config: Vec<u8>,
    layers: bool,
    corrupt_layer: bool,
  }

  impl Default for Artifact {
    fn default() -> Self {
      Self {
        config_media_type: CONFIG_MEDIA_TYPE,
        layer_media_type: "application/wasm",
        config: br#"{"moonlit":{"middlewares":["build","test"]}}"#.to_vec(),
        layers: true,
        corrupt_layer: false,
      }
    }
  }

  fn digest(bytes: &[u8]) -> String {
    format!("sha256:{}", hex::encode(Sha256::digest(bytes)))
  }

  async fn registry(artifact: Artifact) -> Registry {
    let server = MockServer::start().await;
    let host = server.address().to_string();
    let config_digest = digest(&artifact.config);
    let layer_digest = digest(WASM);
    let layers = if artifact.layers {
      serde_json::json!([{ "mediaType": artifact.layer_media_type, "digest": layer_digest, "size": WASM.len() }])
    } else {
      serde_json::json!([])
    };
    let manifest = serde_json::to_vec(&serde_json::json!({
      "schemaVersion": 2,
      "mediaType": MANIFEST_MEDIA_TYPE,
      "config": { "mediaType": artifact.config_media_type, "digest": config_digest, "size": artifact.config.len() },
      "layers": layers,
    }))
    .unwrap();
    let manifest_digest = digest(&manifest);
    let manifest_response = ResponseTemplate::new(200)
      .insert_header("Content-Type", MANIFEST_MEDIA_TYPE)
      .insert_header("Docker-Content-Digest", manifest_digest.as_str())
      .set_body_bytes(manifest);

    Mock::given(method("GET"))
      .and(path("/v2/"))
      .respond_with(ResponseTemplate::new(200))
      .mount(&server)
      .await;
    Mock::given(method("GET"))
      .and(path("/v2/w/plugin/manifests/1.0.0"))
      .respond_with(manifest_response.clone())
      .mount(&server)
      .await;
    Mock::given(method("GET"))
      .and(path(format!("/v2/w/plugin/manifests/{manifest_digest}")))
      .respond_with(manifest_response)
      .mount(&server)
      .await;
    Mock::given(method("GET"))
      .and(path(format!("/v2/w/plugin/blobs/{config_digest}")))
      .respond_with(ResponseTemplate::new(200).set_body_bytes(artifact.config.clone()))
      .mount(&server)
      .await;
    let layer_body = if artifact.corrupt_layer {
      b"\0asm-tampered".to_vec()
    } else {
      WASM.to_vec()
    };
    Mock::given(method("GET"))
      .and(path(format!("/v2/w/plugin/blobs/{layer_digest}")))
      .respond_with(ResponseTemplate::new(200).set_body_bytes(layer_body))
      .mount(&server)
      .await;

    Registry {
      _server: server,
      host,
      manifest_digest,
    }
  }

  async fn failing_registry(status: u16, body: &str) -> (MockServer, String) {
    let server = MockServer::start().await;
    let host = server.address().to_string();
    Mock::given(method("GET"))
      .and(path("/v2/"))
      .respond_with(ResponseTemplate::new(200))
      .mount(&server)
      .await;
    Mock::given(method("GET"))
      .and(path("/v2/w/plugin/manifests/1.0.0"))
      .respond_with(ResponseTemplate::new(status).set_body_string(body))
      .mount(&server)
      .await;
    (server, host)
  }

  fn opts(offline: bool) -> ResolveOptions {
    ResolveOptions {
      offline,
      tag_ttl: Duration::from_mins(15),
    }
  }

  fn cache_at(dir: &tempfile::TempDir, now: &Arc<AtomicU64>) -> Cache {
    Cache::with_root_and_clock(dir.path().to_path_buf(), Box::new(ManualClock(Arc::clone(now))))
  }

  async fn resolve_from(
    host: &str,
    raw_ref: &str,
    offline: bool,
    cache: &Cache,
    progress: Option<ProgressFn<'_>>,
  ) -> Result<ResolvedPlugin, ResolveError> {
    let home = tempfile::tempdir().unwrap();
    let client = OciClient::with_plain_http_for(host);
    resolve_with(&client, home.path(), raw_ref, &opts(offline), cache, progress).await
  }

  #[tokio::test]
  async fn pulls_a_tag_caches_it_and_reports_progress() {
    let registry = registry(Artifact::default()).await;
    let dir = tempfile::tempdir().unwrap();
    let cache = cache_at(&dir, &Arc::new(AtomicU64::new(1_000)));
    let raw_ref = format!("{}/w/plugin:1.0.0", registry.host);
    let seen = Mutex::new(Vec::new());
    let report = |received: u64, total: Option<u64>| seen.lock().unwrap().push((received, total));

    let resolved = resolve_from(&registry.host, &raw_ref, false, &cache, Some(&report))
      .await
      .unwrap();

    assert!(!resolved.cached);
    assert_eq!(resolved.source, format!("oci://{raw_ref}"));
    assert_eq!(resolved.digest.as_deref(), Some(registry.manifest_digest.as_str()));
    assert_eq!(resolved.middlewares, Some(vec!["build".to_string(), "test".to_string()]));
    assert_eq!(std::fs::read(&resolved.wasm_path).unwrap(), WASM);
    assert_eq!(*seen.lock().unwrap(), vec![(WASM.len() as u64, Some(WASM.len() as u64))]);
    assert_eq!(
      cache.read_ref(&raw_ref, Duration::from_mins(15)),
      Some(registry.manifest_digest)
    );
  }

  #[tokio::test]
  async fn a_fresh_tag_is_served_from_the_cache_even_offline() {
    let registry = registry(Artifact::default()).await;
    let dir = tempfile::tempdir().unwrap();
    let cache = cache_at(&dir, &Arc::new(AtomicU64::new(1_000)));
    let raw_ref = format!("{}/w/plugin:1.0.0", registry.host);
    resolve_from(&registry.host, &raw_ref, false, &cache, None).await.unwrap();

    let again = resolve_from(&registry.host, &raw_ref, true, &cache, None).await.unwrap();

    assert!(again.cached);
    assert_eq!(again.middlewares, Some(vec!["build".to_string(), "test".to_string()]));
  }

  #[tokio::test]
  async fn an_expired_tag_is_revalidated_against_the_registry() {
    let registry = registry(Artifact::default()).await;
    let dir = tempfile::tempdir().unwrap();
    let now = Arc::new(AtomicU64::new(1_000));
    let cache = cache_at(&dir, &now);
    let raw_ref = format!("{}/w/plugin:1.0.0", registry.host);
    resolve_from(&registry.host, &raw_ref, false, &cache, None).await.unwrap();
    now.fetch_add(16 * 60, Ordering::SeqCst);

    let offline = resolve_from(&registry.host, &raw_ref, true, &cache, None).await.unwrap_err();
    let online = resolve_from(&registry.host, &raw_ref, false, &cache, None).await.unwrap();

    assert!(matches!(offline, ResolveError::OfflineMiss(_)));
    assert!(!online.cached);
    assert_eq!(online.digest, Some(registry.manifest_digest));
  }

  #[tokio::test]
  async fn a_digest_pinned_reference_skips_the_tag_lookup() {
    let registry = registry(Artifact::default()).await;
    let dir = tempfile::tempdir().unwrap();
    let cache = cache_at(&dir, &Arc::new(AtomicU64::new(1_000)));
    let raw_ref = format!("{}/w/plugin@{}", registry.host, registry.manifest_digest);

    let resolved = resolve_from(&registry.host, &raw_ref, false, &cache, None).await.unwrap();
    let cached = resolve_from(&registry.host, &raw_ref, true, &cache, None).await.unwrap();

    assert!(!resolved.cached);
    assert_eq!(resolved.digest, Some(registry.manifest_digest));
    assert!(cached.cached);
  }

  #[tokio::test]
  async fn offline_without_a_cached_ref_is_an_offline_miss() {
    let dir = tempfile::tempdir().unwrap();
    let cache = cache_at(&dir, &Arc::new(AtomicU64::new(1_000)));

    let err = resolve_from("127.0.0.1:9", "127.0.0.1:9/w/plugin:1.0.0", true, &cache, None)
      .await
      .unwrap_err();

    assert!(
      matches!(&err, ResolveError::OfflineMiss(source) if source == "oci://127.0.0.1:9/w/plugin:1.0.0"),
      "{err:?}"
    );
  }

  #[tokio::test]
  async fn offline_with_a_cached_ref_but_no_plugin_is_an_offline_miss() {
    let dir = tempfile::tempdir().unwrap();
    let cache = cache_at(&dir, &Arc::new(AtomicU64::new(1_000)));
    let raw_ref = "127.0.0.1:9/w/plugin:1.0.0";
    cache.write_ref(raw_ref, "sha256:abc").unwrap();

    let err = resolve_from("127.0.0.1:9", raw_ref, true, &cache, None).await.unwrap_err();

    assert!(matches!(err, ResolveError::OfflineMiss(_)), "{err:?}");
  }

  #[tokio::test]
  async fn an_unparseable_reference_is_invalid() {
    let dir = tempfile::tempdir().unwrap();
    let cache = cache_at(&dir, &Arc::new(AtomicU64::new(1_000)));

    let err = resolve_from("127.0.0.1:9", "::not a ref::", false, &cache, None)
      .await
      .unwrap_err();

    assert!(matches!(err, ResolveError::InvalidReference(_)), "{err:?}");
  }

  async fn resolve_artifact(artifact: Artifact) -> ResolveError {
    let registry = registry(artifact).await;
    let dir = tempfile::tempdir().unwrap();
    let cache = cache_at(&dir, &Arc::new(AtomicU64::new(1_000)));
    let raw_ref = format!("{}/w/plugin:1.0.0", registry.host);
    resolve_from(&registry.host, &raw_ref, false, &cache, None).await.unwrap_err()
  }

  #[tokio::test]
  async fn an_artifact_without_layers_is_a_media_type_mismatch() {
    let err = resolve_artifact(Artifact {
      layers: false,
      ..Artifact::default()
    })
    .await;
    assert!(
      matches!(&err, ResolveError::MediaTypeMismatch(msg) if msg == "artifact has no layers"),
      "{err:?}"
    );
  }

  #[tokio::test]
  async fn a_foreign_config_media_type_is_rejected() {
    let err = resolve_artifact(Artifact {
      config_media_type: "application/vnd.oci.image.config.v1+json",
      ..Artifact::default()
    })
    .await;
    assert!(
      matches!(&err, ResolveError::MediaTypeMismatch(msg) if msg.starts_with("config media type is")),
      "{err:?}"
    );
  }

  #[tokio::test]
  async fn a_foreign_layer_media_type_is_rejected() {
    let err = resolve_artifact(Artifact {
      layer_media_type: "application/vnd.oci.image.layer.v1.tar+gzip",
      ..Artifact::default()
    })
    .await;
    assert!(
      matches!(&err, ResolveError::MediaTypeMismatch(msg) if msg.starts_with("layer media type is")),
      "{err:?}"
    );
  }

  #[tokio::test]
  async fn a_tampered_layer_is_a_digest_mismatch() {
    let err = resolve_artifact(Artifact {
      corrupt_layer: true,
      ..Artifact::default()
    })
    .await;
    assert!(matches!(err, ResolveError::DigestMismatch(_)), "{err:?}");
  }

  #[tokio::test]
  async fn a_config_without_middlewares_resolves_with_none() {
    let registry = registry(Artifact {
      config: b"not json".to_vec(),
      ..Artifact::default()
    })
    .await;
    let dir = tempfile::tempdir().unwrap();
    let cache = cache_at(&dir, &Arc::new(AtomicU64::new(1_000)));
    let raw_ref = format!("{}/w/plugin:1.0.0", registry.host);

    let resolved = resolve_from(&registry.host, &raw_ref, false, &cache, None).await.unwrap();

    assert_eq!(resolved.middlewares, None);
  }

  #[tokio::test]
  async fn failing_to_record_the_ref_is_an_io_error() {
    let registry = registry(Artifact::default()).await;
    let dir = tempfile::tempdir().unwrap();
    let blocker = dir.path().join("not-a-directory");
    std::fs::write(&blocker, b"file").unwrap();
    let cache = Cache::with_root_and_clock(blocker, Box::new(ManualClock(Arc::new(AtomicU64::new(1_000)))));
    let raw_ref = format!("{}/w/plugin:1.0.0", registry.host);

    let err = resolve_from(&registry.host, &raw_ref, false, &cache, None).await.unwrap_err();

    assert!(matches!(err, ResolveError::Io(_)), "{err:?}");
  }

  async fn failing_resolve(status: u16, body: &str) -> ResolveError {
    let (_server, host) = failing_registry(status, body).await;
    let dir = tempfile::tempdir().unwrap();
    let cache = cache_at(&dir, &Arc::new(AtomicU64::new(1_000)));
    let raw_ref = format!("{host}/w/plugin:1.0.0");
    resolve_from(&host, &raw_ref, false, &cache, None).await.unwrap_err()
  }

  #[tokio::test]
  async fn a_401_is_an_auth_error() {
    let err = failing_resolve(401, "").await;
    assert!(matches!(err, ResolveError::Auth(_)), "{err:?}");
  }

  #[tokio::test]
  async fn a_403_is_an_auth_error() {
    let err = failing_resolve(403, "forbidden").await;
    assert!(matches!(err, ResolveError::Auth(_)), "{err:?}");
  }

  #[tokio::test]
  async fn a_plain_404_is_not_found() {
    let err = failing_resolve(404, "gone").await;
    assert!(matches!(err, ResolveError::NotFound(_)), "{err:?}");
  }

  #[tokio::test]
  async fn a_manifest_unknown_envelope_is_not_found() {
    let body = r#"{"errors":[{"code":"MANIFEST_UNKNOWN","message":"manifest unknown"}]}"#;
    let err = failing_resolve(404, body).await;
    assert!(matches!(err, ResolveError::NotFound(_)), "{err:?}");
  }

  #[tokio::test]
  async fn a_denied_envelope_is_an_auth_error() {
    let body = r#"{"errors":[{"code":"DENIED","message":"requested access to the resource is denied"}]}"#;
    let err = failing_resolve(403, body).await;
    assert!(matches!(err, ResolveError::Auth(_)), "{err:?}");
  }

  #[tokio::test]
  async fn other_envelopes_are_network_errors() {
    let body = r#"{"errors":[{"code":"TOOMANYREQUESTS","message":"slow down"}]}"#;
    let err = failing_resolve(429, body).await;
    assert!(matches!(err, ResolveError::Network(_)), "{err:?}");
  }

  #[tokio::test]
  async fn a_server_error_is_a_network_error() {
    let err = failing_resolve(500, "boom").await;
    assert!(matches!(err, ResolveError::Network(_)), "{err:?}");
  }

  #[test]
  fn verify_media_types_accepts_both_wasm_layer_types() {
    assert!(verify_media_types(CONFIG_MEDIA_TYPE, "application/wasm").is_ok());
    assert!(verify_media_types(CONFIG_MEDIA_TYPE, "application/vnd.wasm.content.layer.v1+wasm").is_ok());
  }

  #[test]
  fn parse_middlewares_skips_non_string_entries() {
    let parsed = parse_middlewares(br#"{"moonlit":{"middlewares":["a",1,"b"]}}"#);
    assert_eq!(parsed, Some(vec!["a".to_string(), "b".to_string()]));
    assert_eq!(parse_middlewares(br#"{"moonlit":{}}"#), None);
  }

  #[test]
  fn the_production_client_is_constructible() {
    let _client = OciClient::new();
  }
}
