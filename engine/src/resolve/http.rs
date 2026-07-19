//! `http(s)://` resolution (§4.3): download the component and cache it keyed by `sha256(url)`. URLs
//! are treated as immutable — a cache hit is reused with no revalidation.

use crate::cache::{Cache, PluginMeta};
use crate::resolve::{ProgressFn, ResolveError, ResolveOptions, ResolvedPlugin, sha256_hex};

/// Resolve an `http`/`https` source to a cached component path.
// No non-test caller until Task 8 wires up the scheme dispatcher.
#[allow(dead_code)]
pub(crate) async fn resolve_http(
    url: &str,
    opts: &ResolveOptions,
    cache: &Cache,
    progress: Option<ProgressFn<'_>>,
) -> Result<ResolvedPlugin, ResolveError> {
    let key = sha256_hex(url);

    if cache.has_plugin(&key) {
        return Ok(ResolvedPlugin {
            wasm_path: cache.plugin_wasm(&key),
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
        .store_plugin(&key, &meta, &bytes)
        .map_err(|e| ResolveError::Io(format!("caching {url}: {e}")))?;

    Ok(ResolvedPlugin {
        wasm_path,
        source: url.to_string(),
        digest: None,
        cached: false,
        middlewares: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};

    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::cache::{Cache, Clock};

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

    #[tokio::test]
    async fn downloads_caches_and_reports_progress() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/plugin.wasm"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![0u8, 1, 2, 3]))
            .expect(1) // second call must hit the cache, not the server
            .mount(&server)
            .await;
        let url = format!("{}/plugin.wasm", server.uri());
        let (cache, _d) = cache();
        let opts = ResolveOptions::default();

        let seen = Arc::new(AtomicU64::new(0));
        let seen2 = seen.clone();
        let progress = move |received: u64, _total: Option<u64>| {
            seen2.store(received, Ordering::SeqCst);
        };

        let first = resolve_http(&url, &opts, &cache, Some(&progress))
            .await
            .unwrap();
        assert_eq!(std::fs::read(&first.wasm_path).unwrap(), vec![0, 1, 2, 3]);
        assert_eq!(first.digest, None);
        assert!(!first.cached);
        assert_eq!(seen.load(Ordering::SeqCst), 4);

        // Second resolution is served from cache (mock `.expect(1)` verifies no second request).
        let second = resolve_http(&url, &opts, &cache, None).await.unwrap();
        assert!(second.cached);
        assert_eq!(second.wasm_path, first.wasm_path);
    }

    #[tokio::test]
    async fn offline_miss_errors_without_touching_network() {
        let (cache, _d) = cache();
        let opts = ResolveOptions {
            offline: true,
            ..Default::default()
        };
        let err = resolve_http("http://127.0.0.1:1/plugin.wasm", &opts, &cache, None)
            .await
            .unwrap_err();
        assert!(matches!(err, ResolveError::OfflineMiss(_)));
    }

    #[tokio::test]
    async fn http_404_is_not_found() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/missing.wasm"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        let url = format!("{}/missing.wasm", server.uri());
        let (cache, _d) = cache();
        let err = resolve_http(&url, &ResolveOptions::default(), &cache, None)
            .await
            .unwrap_err();
        assert!(matches!(err, ResolveError::NotFound(_)));
    }
}
