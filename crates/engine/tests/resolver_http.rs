use std::sync::Mutex;
use std::time::Duration;

use moonlit_engine::cache::{Cache, SystemClock};
use moonlit_engine::plugin::resolver::{PluginSource, ProgressFn, ResolveError, ResolveOptions, resolve};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const BODY: &[u8] = b"\0asm-http-plugin";

fn cache_in(dir: &tempfile::TempDir) -> Cache {
  Cache::with_root_and_clock(dir.path().to_path_buf(), Box::new(SystemClock))
}

fn online() -> ResolveOptions {
  ResolveOptions {
    offline: false,
    tag_ttl: Duration::from_mins(15),
  }
}

fn offline() -> ResolveOptions {
  ResolveOptions {
    offline: true,
    tag_ttl: Duration::from_mins(15),
  }
}

async fn serve(status: u16, body: &[u8]) -> MockServer {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .and(path("/plugin.wasm"))
    .respond_with(ResponseTemplate::new(status).set_body_bytes(body.to_vec()))
    .mount(&server)
    .await;
  server
}

#[tokio::test]
async fn downloads_caches_and_reports_progress() {
  let server = serve(200, BODY).await;
  let dir = tempfile::tempdir().unwrap();
  let cache = cache_in(&dir);
  let url = format!("{}/plugin.wasm", server.uri());

  let seen = Mutex::new(Vec::new());
  let report = |received: u64, total: Option<u64>| seen.lock().unwrap().push((received, total));
  let progress: ProgressFn = &report;

  let resolved = resolve(&PluginSource::Http(url.clone()), &online(), &cache, Some(progress))
    .await
    .unwrap();

  assert!(!resolved.cached);
  assert_eq!(resolved.source, url);
  assert_eq!(resolved.digest, None);
  assert_eq!(resolved.middlewares, None);
  assert_eq!(std::fs::read(&resolved.wasm_path).unwrap(), BODY);
  let last = *seen.lock().unwrap().last().unwrap();
  assert_eq!(last, (BODY.len() as u64, Some(BODY.len() as u64)));
}

#[tokio::test]
async fn second_resolve_is_served_from_the_cache() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .and(path("/plugin.wasm"))
    .respond_with(ResponseTemplate::new(200).set_body_bytes(BODY.to_vec()))
    .expect(1)
    .mount(&server)
    .await;
  let dir = tempfile::tempdir().unwrap();
  let cache = cache_in(&dir);
  let source = PluginSource::Http(format!("{}/plugin.wasm", server.uri()));

  let first = resolve(&source, &online(), &cache, None).await.unwrap();
  let second = resolve(&source, &offline(), &cache, None).await.unwrap();

  assert!(!first.cached);
  assert!(second.cached);
  assert_eq!(first.wasm_path, second.wasm_path);
  let listed = cache.list();
  assert_eq!(listed.len(), 1);
  assert_eq!(listed[0].1.source, format!("{}/plugin.wasm", server.uri()));
}

#[tokio::test]
async fn offline_without_a_cached_copy_is_an_offline_miss() {
  let dir = tempfile::tempdir().unwrap();
  let cache = cache_in(&dir);
  let url = "http://127.0.0.1:9/plugin.wasm".to_string();

  let err = resolve(&PluginSource::Http(url.clone()), &offline(), &cache, None)
    .await
    .unwrap_err();

  let ResolveError::OfflineMiss(missed) = err else {
    panic!("expected OfflineMiss, got {err:?}");
  };
  assert_eq!(missed, url);
}

#[tokio::test]
async fn http_404_is_not_found() {
  let server = serve(404, b"").await;
  let dir = tempfile::tempdir().unwrap();
  let source = PluginSource::Http(format!("{}/plugin.wasm", server.uri()));

  let err = resolve(&source, &online(), &cache_in(&dir), None).await.unwrap_err();

  let ResolveError::NotFound(msg) = err else {
    panic!("expected NotFound, got {err:?}");
  };
  assert!(msg.contains("404"), "{msg}");
}

#[tokio::test]
async fn other_http_errors_are_network_errors() {
  let server = serve(500, b"boom").await;
  let dir = tempfile::tempdir().unwrap();
  let source = PluginSource::Http(format!("{}/plugin.wasm", server.uri()));

  let err = resolve(&source, &online(), &cache_in(&dir), None).await.unwrap_err();

  let ResolveError::Network(msg) = err else {
    panic!("expected Network, got {err:?}");
  };
  assert!(msg.contains("500"), "{msg}");
}

#[tokio::test]
async fn unreachable_host_is_a_network_error() {
  let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
  let url = format!("http://{}/plugin.wasm", listener.local_addr().unwrap());
  drop(listener);
  let dir = tempfile::tempdir().unwrap();

  let err = resolve(&PluginSource::Http(url), &online(), &cache_in(&dir), None)
    .await
    .unwrap_err();

  let ResolveError::Network(msg) = err else {
    panic!("expected Network, got {err:?}");
  };
  assert!(msg.starts_with("GET "), "{msg}");
}

#[tokio::test]
async fn failing_to_write_the_cache_is_an_io_error() {
  let server = serve(200, BODY).await;
  let dir = tempfile::tempdir().unwrap();
  let blocker = dir.path().join("not-a-directory");
  std::fs::write(&blocker, b"file").unwrap();
  let cache = Cache::with_root_and_clock(blocker, Box::new(SystemClock));
  let source = PluginSource::Http(format!("{}/plugin.wasm", server.uri()));

  let err = resolve(&source, &online(), &cache, None).await.unwrap_err();

  let ResolveError::Io(msg) = err else {
    panic!("expected Io, got {err:?}");
  };
  assert!(msg.starts_with("caching "), "{msg}");
}
