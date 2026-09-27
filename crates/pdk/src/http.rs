//! Tiny blocking HTTP client over the host `http_send` capability. Verbs, bearer
//! auth, custom headers, JSON in/out, per-request timeout, and gzip response
//! inflate. Sized for GitHub/GitLab/Slack/OpenAI/npm/NuGet-style REST calls.

use std::io::Read;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::context::Host;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HttpMethod {
  Get,
  Post,
  Put,
  Patch,
  Delete,
}

/// Fully-formed request crossing the `Host` boundary (plain-Rust DTO).
#[derive(Clone)]
pub struct HttpRequestData {
  pub method: HttpMethod,
  pub scheme: String,
  pub authority: String,
  pub path_with_query: String,
  pub headers: Vec<(String, String)>,
  pub body: Option<Vec<u8>>,
  pub timeout_ms: Option<u64>,
}

/// Raw response from the host: `body` is exactly the bytes received (still
/// gzipped if the server gzipped it — the ergonomic layer inflates).
pub struct HttpResponseData {
  pub status: u16,
  pub headers: Vec<(String, String)>,
  pub body: Vec<u8>,
}

/// HTTP entry point, created via `ctx.http()`.
pub struct Http<'a> {
  host: &'a dyn Host,
}

impl<'a> Http<'a> {
  #[must_use]
  pub(crate) fn new(host: &'a dyn Host) -> Self {
    Self { host }
  }
  pub fn get(&self, url: impl Into<String>) -> Request<'a> {
    self.request(HttpMethod::Get, url)
  }
  pub fn post(&self, url: impl Into<String>) -> Request<'a> {
    self.request(HttpMethod::Post, url)
  }
  pub fn put(&self, url: impl Into<String>) -> Request<'a> {
    self.request(HttpMethod::Put, url)
  }
  pub fn patch(&self, url: impl Into<String>) -> Request<'a> {
    self.request(HttpMethod::Patch, url)
  }
  pub fn delete(&self, url: impl Into<String>) -> Request<'a> {
    self.request(HttpMethod::Delete, url)
  }
  fn request(&self, method: HttpMethod, url: impl Into<String>) -> Request<'a> {
    Request {
      host: self.host,
      method,
      url: url.into(),
      headers: Vec::new(),
      body: None,
      timeout_ms: None,
      error: None,
    }
  }
}

/// Fluent request builder. Terminal: `send()`.
#[must_use = "a request does nothing until `.send()` is called"]
pub struct Request<'a> {
  host: &'a dyn Host,
  method: HttpMethod,
  url: String,
  headers: Vec<(String, String)>,
  body: Option<Vec<u8>>,
  timeout_ms: Option<u64>,
  /// Deferred error (e.g. JSON serialize) surfaced by `send()`.
  error: Option<String>,
}

impl Request<'_> {
  pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
    self.headers.push((name.into(), value.into()));
    self
  }
  pub fn bearer(mut self, token: impl AsRef<str>) -> Self {
    self
      .headers
      .push(("authorization".to_string(), format!("Bearer {}", token.as_ref())));
    self
  }
  pub fn json<T: Serialize>(mut self, value: &T) -> Self {
    match serde_json::to_vec(value) {
      Ok(bytes) => {
        self
          .headers
          .push(("content-type".to_string(), "application/json".to_string()));
        self.body = Some(bytes);
      }
      Err(e) => self.error = Some(format!("request json serialize: {e}")),
    }
    self
  }
  pub fn body_bytes(mut self, bytes: Vec<u8>) -> Self {
    self.body = Some(bytes);
    self
  }
  pub fn timeout_ms(mut self, ms: u64) -> Self {
    self.timeout_ms = Some(ms);
    self
  }

  pub fn send(self) -> Result<Response, String> {
    if let Some(e) = self.error {
      return Err(e);
    }
    let (scheme, authority, path_with_query) = parse_url(&self.url)?;
    let mut headers = self.headers;
    if !headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("accept-encoding")) {
      headers.push(("accept-encoding".to_string(), "gzip".to_string()));
    }
    let req = HttpRequestData {
      method: self.method,
      scheme,
      authority,
      path_with_query,
      headers,
      body: self.body,
      timeout_ms: self.timeout_ms,
    };
    let raw = self.host.http_send(&req)?;
    Response::from_raw(raw)
  }
}

/// A received HTTP response (body already gzip-inflated when applicable).
pub struct Response {
  status: u16,
  headers: Vec<(String, String)>,
  body: Vec<u8>,
}

impl Response {
  fn from_raw(raw: HttpResponseData) -> Result<Self, String> {
    let gzipped = raw
      .headers
      .iter()
      .any(|(k, v)| k.eq_ignore_ascii_case("content-encoding") && v.to_ascii_lowercase().contains("gzip"));
    let body = if gzipped {
      let mut out = Vec::new();
      flate2::read::GzDecoder::new(&raw.body[..])
        .read_to_end(&mut out)
        .map_err(|e| format!("gzip inflate: {e}"))?;
      out
    } else {
      raw.body
    };
    Ok(Self {
      status: raw.status,
      headers: raw.headers,
      body,
    })
  }
  #[must_use]
  pub fn status(&self) -> u16 {
    self.status
  }
  #[must_use]
  pub fn is_success(&self) -> bool {
    (200..=299).contains(&self.status)
  }
  #[must_use]
  pub fn header(&self, name: &str) -> Option<&str> {
    self
      .headers
      .iter()
      .find(|(k, _)| k.eq_ignore_ascii_case(name))
      .map(|(_, v)| v.as_str())
  }
  #[must_use]
  pub fn bytes(&self) -> &[u8] {
    &self.body
  }
  pub fn text(&self) -> Result<String, String> {
    String::from_utf8(self.body.clone()).map_err(|e| format!("response utf-8: {e}"))
  }
  pub fn json<T: DeserializeOwned>(&self) -> Result<T, String> {
    serde_json::from_slice(&self.body).map_err(|e| format!("response json: {e}"))
  }
}

/// Split `scheme://authority/path?query` into parts. Minimal by design: callers
/// pass absolute URLs that include a path.
fn parse_url(url: &str) -> Result<(String, String, String), String> {
  let (scheme, rest) = url
    .split_once("://")
    .ok_or_else(|| format!("invalid url (no scheme): {url}"))?;
  if scheme.is_empty() {
    return Err(format!("invalid url (empty scheme): {url}"));
  }
  let (authority, path_and_query) = match rest.find('/') {
    Some(i) => (&rest[..i], &rest[i..]),
    None => (rest, "/"),
  };
  if authority.is_empty() {
    return Err(format!("invalid url (empty authority): {url}"));
  }
  Ok((scheme.to_string(), authority.to_string(), path_and_query.to_string()))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::Context;
  use crate::testing::MockHost;

  #[test]
  fn get_builds_request_and_reads_json() {
    #[derive(serde::Deserialize)]
    struct R {
      tag: String,
    }
    let host = MockHost::new().with_http_response(200, br#"{"tag":"v1"}"#);
    let ctx = Context::new(&host, "/w".into(), "s".into());
    let resp = ctx
      .http()
      .get("https://api.example.com/repos?page=1")
      .bearer("tok")
      .send()
      .unwrap();
    assert!(resp.is_success());
    let r: R = resp.json().unwrap();
    assert_eq!(r.tag, "v1");

    let reqs = host.recorded_requests();
    assert_eq!(reqs[0].method, HttpMethod::Get);
    assert_eq!(reqs[0].scheme, "https");
    assert_eq!(reqs[0].authority, "api.example.com");
    assert_eq!(reqs[0].path_with_query, "/repos?page=1");
    assert!(reqs[0].headers.iter().any(|(k, v)| k == "authorization" && v == "Bearer tok"));
    assert!(
      reqs[0]
        .headers
        .iter()
        .any(|(k, v)| k.eq_ignore_ascii_case("accept-encoding") && v == "gzip")
    );
  }

  #[test]
  fn post_json_sets_body_and_content_type() {
    let host = MockHost::new().with_http_response(201, b"{}");
    let ctx = Context::new(&host, "/w".into(), "s".into());
    let body = serde_json::json!({ "name": "x" });
    let _ = ctx.http().post("https://h/api").json(&body).send().unwrap();
    let reqs = host.recorded_requests();
    assert_eq!(reqs[0].method, HttpMethod::Post);
    assert_eq!(reqs[0].body.as_deref().unwrap(), br#"{"name":"x"}"#);
    assert!(
      reqs[0]
        .headers
        .iter()
        .any(|(k, v)| k == "content-type" && v == "application/json")
    );
  }

  #[test]
  fn verbs_map_correctly() {
    let host = MockHost::new()
      .with_http_response(200, b"")
      .with_http_response(200, b"")
      .with_http_response(200, b"");
    let ctx = Context::new(&host, "/w".into(), "s".into());
    ctx.http().put("https://h/a").send().unwrap();
    ctx.http().patch("https://h/b").send().unwrap();
    ctx.http().delete("https://h/c").send().unwrap();
    let methods: Vec<_> = host.recorded_requests().iter().map(|r| r.method).collect();
    assert_eq!(methods, vec![HttpMethod::Put, HttpMethod::Patch, HttpMethod::Delete]);
  }

  #[test]
  fn gzip_response_is_inflated() {
    use std::io::Write;
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    enc.write_all(b"hello gzip").unwrap();
    let gz = enc.finish().unwrap();
    let host = MockHost::new().with_http_response_headers(200, vec![("content-encoding".to_string(), "gzip".to_string())], &gz);
    let ctx = Context::new(&host, "/w".into(), "s".into());
    let resp = ctx.http().get("https://h/x").send().unwrap();
    assert_eq!(resp.text().unwrap(), "hello gzip");
  }

  #[test]
  fn http_error_surfaces_from_send() {
    let host = MockHost::new().with_http_error("network: host not permitted");
    let ctx = Context::new(&host, "/w".into(), "s".into());
    let e = ctx.http().get("https://h/x").send().err().expect("expected error");
    assert!(e.contains("not permitted"), "got: {e}");
  }

  fn send_err(url: &str) -> String {
    let host = MockHost::new().with_http_response(200, b"");
    let ctx = Context::new(&host, "/w".into(), "s".into());
    ctx.http().get(url).send().err().expect("expected url error")
  }

  #[test]
  fn bad_url_is_err() {
    assert!(send_err("no-scheme").contains("invalid url (no scheme)"));
    assert!(send_err("://h/x").contains("invalid url (empty scheme)"));
    assert!(send_err("https:///x").contains("invalid url (empty authority)"));
  }

  #[test]
  fn url_without_a_path_requests_the_root() {
    let host = MockHost::new().with_http_response(200, b"");
    let ctx = Context::new(&host, "/w".into(), "s".into());
    ctx.http().get("https://h").send().unwrap();
    assert_eq!(host.recorded_requests()[0].path_with_query, "/");
  }

  #[test]
  fn builder_sets_headers_body_and_timeout() {
    let host = MockHost::new().with_http_response(204, b"");
    let ctx = Context::new(&host, "/w".into(), "s".into());
    ctx
      .http()
      .put("https://h/up")
      .header("x-trace", "1")
      .header("Accept-Encoding", "identity")
      .body_bytes(b"raw".to_vec())
      .timeout_ms(250)
      .send()
      .unwrap();
    let req = &host.recorded_requests()[0];
    assert_eq!(req.body.as_deref(), Some(&b"raw"[..]));
    assert_eq!(req.timeout_ms, Some(250));
    assert!(req.headers.iter().any(|(k, v)| k == "x-trace" && v == "1"));
    assert!(!req.headers.iter().any(|(_, v)| v == "gzip"));
  }

  #[test]
  fn json_body_that_cannot_serialize_fails_at_send() {
    let host = MockHost::new().with_http_response(200, b"");
    let ctx = Context::new(&host, "/w".into(), "s".into());
    let body: std::collections::HashMap<(i32, i32), i32> = std::collections::HashMap::from([((1, 2), 3)]);
    let e = ctx
      .http()
      .post("https://h/api")
      .json(&body)
      .send()
      .err()
      .expect("expected serialize error");
    assert!(e.starts_with("request json serialize:"), "got: {e}");
    assert!(host.recorded_requests().is_empty());
  }

  #[test]
  fn response_exposes_status_headers_and_bytes() {
    let host = MockHost::new().with_http_response_headers(404, vec![("X-Id".to_string(), "abc".to_string())], b"gone");
    let ctx = Context::new(&host, "/w".into(), "s".into());
    let resp = ctx.http().get("https://h/x").send().unwrap();
    assert_eq!(resp.status(), 404);
    assert!(!resp.is_success());
    assert_eq!(resp.header("x-id"), Some("abc"));
    assert_eq!(resp.header("missing"), None);
    assert_eq!(resp.bytes(), b"gone");
  }

  #[test]
  fn response_text_and_json_report_decode_errors() {
    let host = MockHost::new().with_http_response(200, &[0xff, 0xfe]);
    let ctx = Context::new(&host, "/w".into(), "s".into());
    let resp = ctx.http().get("https://h/x").send().unwrap();
    assert!(resp.text().expect_err("utf-8 error").starts_with("response utf-8:"));
    assert!(
      resp
        .json::<serde_json::Value>()
        .expect_err("json error")
        .starts_with("response json:")
    );
  }
}
