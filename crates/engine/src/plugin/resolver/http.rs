use crate::cache::{Cache, PluginMeta, url_key};
use std::time::Duration;

use crate::plugin::resolver::{CONNECT_TIMEOUT, ProgressFn, READ_TIMEOUT, ResolveError, ResolveOptions, ResolvedPlugin};
use crate::redirect;

/// Why a redirect is not followed.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
enum RedirectRefusal {
  /// A download that started on https was sent to another scheme. Holds the scheme and host, never the path or query,
  /// which can carry a token.
  #[error("refusing to follow a redirect from https to {0}; the download must stay on https")]
  Downgrade(String),
  /// The download went through more redirects than [`redirect::MAX_REDIRECTS`].
  #[error("too many redirects")]
  TooMany,
}

/// Decides whether to follow a redirect to `next`, given the URLs already requested (the first one, then each hop).
fn check_redirect(previous: &[reqwest::Url], next: &reqwest::Url) -> Result<(), RedirectRefusal> {
  if redirect::too_many(previous) {
    return Err(RedirectRefusal::TooMany);
  }
  if redirect::leaves_https(previous, next) {
    return Err(RedirectRefusal::Downgrade(format!(
      "{}://{}",
      next.scheme(),
      next.host_str().unwrap_or("")
    )));
  }
  Ok(())
}

fn download_client(connect: Duration, read: Duration) -> reqwest::Client {
  let policy = reqwest::redirect::Policy::custom(|attempt| match check_redirect(attempt.previous(), attempt.url()) {
    Ok(()) => attempt.follow(),
    Err(refusal) => attempt.error(refusal),
  });
  reqwest::Client::builder()
    .connect_timeout(connect)
    .read_timeout(read)
    .redirect(policy)
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
    .map_err(|e| match redirect::refusal::<RedirectRefusal>(&e) {
      Some(refusal) => ResolveError::Network(format!("GET {url}: {refusal}")),
      None => ResolveError::Network(format!("GET {url}: {e}")),
    })?;

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
  use crate::plugin::resolver::tests::silent_server;

  #[tokio::test]
  async fn a_server_that_accepts_and_never_answers_is_a_network_error() {
    let (addr, held) = silent_server().await;
    let url = format!("http://{addr}/plugin.wasm");
    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::with_root_and_clock(dir.path().to_path_buf(), Box::new(SystemClock));
    let opts = ResolveOptions::default();
    let client = download_client(Duration::from_millis(200), Duration::from_millis(200));

    let started = std::time::Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(5), resolve_http_with(&client, &url, &opts, &cache, None)).await;
    held.abort();

    let err = result.expect("resolve returned before the outer timeout").unwrap_err();
    assert!(matches!(err, ResolveError::Network(_)), "{err:?}");
    assert!(started.elapsed() < Duration::from_secs(3), "{:?}", started.elapsed());
  }

  fn urls(list: &[&str]) -> Vec<reqwest::Url> {
    list.iter().map(|u| reqwest::Url::parse(u).unwrap()).collect()
  }

  fn url(u: &str) -> reqwest::Url {
    reqwest::Url::parse(u).unwrap()
  }

  #[test]
  fn a_redirect_from_https_to_http_is_refused_naming_only_scheme_and_host() {
    let refusal = check_redirect(
      &urls(&["https://cdn.example.com/p.wasm"]),
      &url("http://evil.example.org/p.wasm?token=s3cret"),
    )
    .unwrap_err();
    assert_eq!(refusal, RedirectRefusal::Downgrade("http://evil.example.org".to_string()));
    let shown = refusal.to_string();
    assert!(shown.contains("http://evil.example.org"), "{shown}");
    assert!(!shown.contains("s3cret") && !shown.contains("/p.wasm"), "{shown}");
  }

  #[test]
  fn a_redirect_to_http_after_an_https_hop_is_refused() {
    let previous = urls(&["http://a.example.com/p", "https://b.example.com/p"]);
    assert!(check_redirect(&previous, &url("http://c.example.com/p")).is_err());
  }

  #[test]
  fn redirects_that_do_not_leave_https_or_that_start_on_http_are_followed() {
    assert!(check_redirect(&urls(&["https://a.example.com/p"]), &url("https://b.example.com/p")).is_ok());
    assert!(check_redirect(&urls(&["http://a.example.com/p"]), &url("http://b.example.com/p")).is_ok());
    assert!(check_redirect(&urls(&["http://a.example.com/p"]), &url("https://b.example.com/p")).is_ok());
  }

  #[test]
  fn the_eleventh_redirect_stops() {
    let hops: Vec<reqwest::Url> = (0..=redirect::MAX_REDIRECTS)
      .map(|n| url(&format!("https://h{n}.example.com/p")))
      .collect();
    assert!(check_redirect(&hops[..redirect::MAX_REDIRECTS], &url("https://next.example.com/p")).is_ok());
    assert_eq!(
      check_redirect(&hops, &url("https://next.example.com/p")),
      Err(RedirectRefusal::TooMany)
    );
  }

  #[tokio::test]
  async fn an_http_to_http_redirect_still_downloads() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    let server = MockServer::start().await;
    Mock::given(method("GET"))
      .and(path("/first"))
      .respond_with(ResponseTemplate::new(302).insert_header("location", "/second"))
      .mount(&server)
      .await;
    Mock::given(method("GET"))
      .and(path("/second"))
      .respond_with(ResponseTemplate::new(200).set_body_bytes(b"wasm".to_vec()))
      .mount(&server)
      .await;
    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::with_root_and_clock(dir.path().to_path_buf(), Box::new(SystemClock));
    let opts = ResolveOptions::default();
    let client = download_client(Duration::from_secs(5), Duration::from_secs(5));

    let resolved = resolve_http_with(&client, &format!("{}/first", server.uri()), &opts, &cache, None)
      .await
      .expect("the redirect is followed");
    assert_eq!(std::fs::read(resolved.wasm_path).unwrap(), b"wasm");
  }

  #[tokio::test]
  async fn a_chain_of_redirects_longer_than_the_limit_is_refused() {
    use wiremock::matchers::{method, path_regex};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};
    let server = MockServer::start().await;
    Mock::given(method("GET"))
      .and(path_regex(r"^/r\d+$"))
      .respond_with(|request: &Request| {
        let n: usize = request.url.path().trim_start_matches("/r").parse().unwrap();
        ResponseTemplate::new(302).insert_header("location", format!("/r{}", n + 1))
      })
      .mount(&server)
      .await;
    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::with_root_and_clock(dir.path().to_path_buf(), Box::new(SystemClock));
    let opts = ResolveOptions::default();
    let client = download_client(Duration::from_secs(5), Duration::from_secs(5));

    let err = resolve_http_with(&client, &format!("{}/r0", server.uri()), &opts, &cache, None)
      .await
      .unwrap_err();
    let ResolveError::Network(message) = err else {
      panic!("{err:?}")
    };
    assert!(message.ends_with(": too many redirects"), "{message}");
  }
}
