use crate::cache::{Cache, PluginMeta, url_key};
use std::time::Duration;

use crate::plugin::resolver::{CONNECT_TIMEOUT, ProgressFn, READ_TIMEOUT, ResolveError, ResolveOptions, ResolvedPlugin};

fn download_client(connect: Duration, read: Duration) -> reqwest::Client {
  reqwest::Client::builder()
    .connect_timeout(connect)
    .read_timeout(read)
    .build()
    .expect("the reqwest client builds with static timeouts and the default TLS backend")
}

pub async fn resolve_http(
  url: &str,
  opts: &ResolveOptions,
  cache: &Cache,
  progress: Option<ProgressFn<'_>>,
) -> Result<ResolvedPlugin, ResolveError> {
  let client = download_client(CONNECT_TIMEOUT, READ_TIMEOUT);
  resolve_http_with(&client, url, opts, cache, progress).await
}

async fn resolve_http_with(
  client: &reqwest::Client,
  url: &str,
  opts: &ResolveOptions,
  cache: &Cache,
  progress: Option<ProgressFn<'_>>,
) -> Result<ResolvedPlugin, ResolveError> {
  let key = url_key(url);
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

  let mut response = client
    .get(url)
    .send()
    .await
    .map_err(|e| ResolveError::Network(format!("GET {url}: {e}")))?;

  let status = response.status();
  if status == reqwest::StatusCode::NOT_FOUND {
    return Err(ResolveError::NotFound(format!("HTTP 404 for {url}")));
  }
  if !status.is_success() {
    return Err(ResolveError::Registry(format!("HTTP {status} for {url}")));
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
  use crate::cache::SystemClock;

  #[tokio::test]
  async fn a_server_that_accepts_and_never_answers_is_a_network_error() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/plugin.wasm", listener.local_addr().unwrap());
    let held = tokio::spawn(async move {
      let mut open = Vec::new();
      while let Ok((stream, _)) = listener.accept().await {
        open.push(stream);
      }
    });
    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::with_root_and_clock(dir.path().to_path_buf(), Box::new(SystemClock));
    let opts = ResolveOptions {
      offline: false,
      tag_ttl: Duration::from_mins(15),
    };
    let client = download_client(Duration::from_millis(200), Duration::from_millis(200));

    let started = std::time::Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(5), resolve_http_with(&client, &url, &opts, &cache, None)).await;
    held.abort();

    let err = result.expect("resolve returned before the outer timeout").unwrap_err();
    assert!(matches!(err, ResolveError::Network(_)), "{err:?}");
    assert!(started.elapsed() < Duration::from_secs(3), "{:?}", started.elapsed());
  }
}
