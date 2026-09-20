use crate::plugin::resolver::cache::{Cache, PluginMeta};
use crate::plugin::resolver::{ProgressFn, ResolveError, ResolveOptions, ResolvedPlugin};

pub async fn resolve_http(
    url: &str,
    opts: &ResolveOptions,
    cache: &Cache,
    progress: Option<ProgressFn<'_>>,
) -> Result<ResolvedPlugin, ResolveError> {
    if cache.has_plugin(url) {
        return Ok(ResolvedPlugin {
            wasm_path: cache.plugin_wasm(url),
            source: url.to_string(),
            digest: None,
            cached: true,
            middlewares: None,
        });
    }

    if opts.offline {
        return Err(ResolveError::OfflineMiss(url.to_string()));
    }

    let mut response = reqwest::get(url)
        .await
        .map_err(|e| ResolveError::Network(format!("GET {url}: {e}")))?;

    let status = response.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        return Err(ResolveError::NotFound(format!("HTTP 404 for {url}")));
    }
    if !status.is_success() {
        return Err(ResolveError::Network(format!("HTTP {status} for {url}")));
    }

    let total = response.content_length();
    let mut bytes: Vec<u8> = Vec::new();
    let mut received: u64 = 0;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| ResolveError::Network(format!("reading body of {url}: {e}")))?
    {
        received += chunk.len() as u64;
        bytes.extend_from_slice(&chunk);
        if let Some(report) = progress {
            report(received, total);
        }
    }

    let meta = PluginMeta {
        source: url.to_string(),
        digest: None,
        layer_digest: None,
        size: bytes.len() as u64,
        pulled_at: cache.now_unix(),
        middlewares: None,
    };
    let wasm_path = cache
        .store_plugin(url, &meta, &bytes)
        .map_err(|e| ResolveError::Io(format!("caching {url}: {e}")))?;

    Ok(ResolvedPlugin {
        wasm_path,
        source: url.to_string(),
        digest: None,
        cached: false,
        middlewares: None,
    })
}
