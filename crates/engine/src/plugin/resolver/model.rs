use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginSource {
  Oci(String),
  File(PathBuf),
  Http(String),
}

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

  #[error("the server answered with an error while resolving plugin: {0}")]
  #[diagnostic(code(moonlit::resolve::registry))]
  Registry(String),

  #[error("offline: no cached plugin for {0}")]
  #[diagnostic(code(moonlit::resolve::offline_miss))]
  OfflineMiss(String),

  #[error("cache I/O error: {0}")]
  #[diagnostic(code(moonlit::resolve::io))]
  Io(String),
}

impl FromStr for PluginSource {
  type Err = ResolveError;

  fn from_str(url: &str) -> Result<Self, Self::Err> {
    let url = url.trim();
    if let Some(rest) = strip_scheme(url, "oci://") {
      return Ok(PluginSource::Oci(rest.to_string()));
    }
    if let Some(rest) = strip_scheme(url, "file://") {
      return Ok(PluginSource::File(PathBuf::from(rest)));
    }
    if strip_scheme(url, "http://").is_some() || strip_scheme(url, "https://").is_some() {
      return Ok(PluginSource::Http(url.to_string()));
    }
    if let Some(scheme) = url.split_once("://").map(|(s, _)| s) {
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

#[derive(Debug, Clone)]
pub struct ResolveOptions {
  pub offline: bool,
  pub tag_ttl: Duration,
}

impl Default for ResolveOptions {
  fn default() -> Self {
    Self {
      offline: false,
      tag_ttl: Duration::from_mins(15),
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPlugin {
  pub wasm_path: PathBuf,
  pub source: String,
  pub digest: Option<String>,
  pub cached: bool,
  pub middlewares: Option<Vec<String>>,
}

pub type ProgressFn<'a> = &'a (dyn Fn(u64, Option<u64>) + Send + Sync);

fn strip_scheme<'a>(url: &'a str, scheme: &str) -> Option<&'a str> {
  url
    .get(..scheme.len())
    .filter(|head| head.eq_ignore_ascii_case(scheme))
    .map(|_| &url[scheme.len()..])
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn plugin_source_reads_the_scheme_in_any_case() {
    assert_eq!(
      "OCI://h.io/acme/x:1.0.0".parse::<PluginSource>().unwrap(),
      PluginSource::Oci("h.io/acme/x:1.0.0".into())
    );
    assert_eq!(
      "File:///tmp/p.wasm".parse::<PluginSource>().unwrap(),
      PluginSource::File(PathBuf::from("/tmp/p.wasm"))
    );
    assert_eq!(
      "HTTPS://h.io/p.wasm".parse::<PluginSource>().unwrap(),
      PluginSource::Http("HTTPS://h.io/p.wasm".into())
    );
    assert!(matches!(
      "ftp://h.io/p.wasm".parse::<PluginSource>(),
      Err(ResolveError::UnsupportedScheme { .. })
    ));
    assert!(matches!(
      "oci:/h.io/acme/x:1".parse::<PluginSource>(),
      Err(ResolveError::InvalidReference(_))
    ));
  }
}
