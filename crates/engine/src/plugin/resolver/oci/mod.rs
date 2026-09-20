mod auth;
mod client;

use crate::plugin::resolver::cache::{Cache, PluginMeta};
use crate::plugin::resolver::oci::client::OciClient;
use crate::plugin::resolver::{ProgressFn, ResolveError, ResolveOptions, ResolvedPlugin};
use oci_client::Reference;
use oci_client::manifest::OciImageManifest;

const CONFIG_MEDIA_TYPE: &str = "application/vnd.wasm.config.v0+json";
const LAYER_MEDIA_TYPES: [&str; 2] = [
    "application/wasm",
    "application/vnd.wasm.content.layer.v1+wasm",
];

pub async fn resolve_oci(
    raw_ref: &str,
    opts: &ResolveOptions,
    cache: &Cache,
    progress: Option<ProgressFn<'_>>,
) -> Result<ResolvedPlugin, ResolveError> {
    let reference: oci_client::Reference = raw_ref
        .parse()
        .map_err(|e| ResolveError::InvalidReference(format!("'{raw_ref}': {e}")))?;
    let home = dirs::home_dir().unwrap_or_default();
    let auth = auth::resolve_auth(reference.registry(), &home);
    let client = OciClient::new();

    let reference: Reference = raw_ref
        .parse()
        .map_err(|e| ResolveError::InvalidReference(format!("'{raw_ref}': {e}")))?;
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

fn parse_middlewares(config_json: &[u8]) -> Option<Vec<String>> {
    let value: serde_json::Value = serde_json::from_slice(config_json).ok()?;
    let arr = value.get("moonlit")?.get("middlewares")?.as_array()?;
    Some(
        arr.iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
    )
}
