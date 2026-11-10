mod model;

use std::time::Duration;

use semver::Version;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

pub use model::{MiddlewareSummary, PackageHit, VerificationStatus, VersionDetail, VersionSummary};

const SEARCH_QUERY: &str = "query search($filter: PackageFilterInput, $paging: PagingInput) { \
packages(filter: $filter, paging: $paging) { items { orgSlug name description latestVersion status middlewares } } }";

const PACKAGE_VERSION_QUERY: &str = "query packageVersion($org: String!, $name: String!, $version: String!) { \
packageVersion(org: $org, name: $name, version: $version) { version worldVersion status middlewares { name description } } }";

const VERSIONS_QUERY: &str = "query versions($org: String!, $name: String!) { \
package(org: $org, name: $name) { versions { version yanked status } } }";

/// A failure talking to the registry.
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
  /// The server answered with a non-success status.
  #[error("registry request failed: {0}")]
  Http(String),
  /// The request could not reach the server or the connection broke before the answer was complete, including timeouts.
  #[error("registry request failed: {0}")]
  Unreachable(String),
  /// The server answered with a GraphQL error; holds the first message.
  #[error("registry returned an error: {0}")]
  GraphQl(String),
  /// The response body did not have the expected shape.
  #[error("registry response could not be decoded: {0}")]
  Decode(String),
  /// The host answered, but not as a Moonlit registry: no GraphQL endpoint, or a body that is not a GraphQL response.
  #[error("{0} is not a Moonlit registry")]
  NotARegistry(String),
  /// The host refused the anonymous GraphQL request with HTTP 401 or 403, so the registry cannot be asked.
  #[error("{url} answered {status} on /graphql")]
  Denied {
    /// The GraphQL endpoint that refused the request.
    url: String,
    /// The HTTP status it answered with.
    status: u16,
  },
  /// The registry URL, or a redirect from it, is plain `http://` to a host that is not loopback, which would send
  /// requests unencrypted.
  #[error("refusing to send registry requests over plain HTTP to {0}; use https:// (http:// is only allowed for localhost)")]
  Insecure(String),
}

const MAX_CANDIDATES: usize = 10;

/// How long a registry request may take to connect, and in all, before it fails.
pub const REGISTRY_TIMEOUT: Duration = Duration::from_secs(10);

const MAX_REDIRECTS: usize = 10;

fn has_scheme(url: &str, prefix: &str) -> bool {
  url.get(..prefix.len()).is_some_and(|head| head.eq_ignore_ascii_case(prefix))
}

fn plain_http_to_remote(url: &reqwest::Url) -> bool {
  url.scheme() == "http" && !url.host_str().is_some_and(is_loopback)
}

fn refused_redirect(error: &reqwest::Error) -> Option<RegistryError> {
  let mut source = std::error::Error::source(error);
  while let Some(cause) = source {
    if let Some(RegistryError::Insecure(url)) = cause.downcast_ref::<RegistryError>() {
      return Some(RegistryError::Insecure(url.clone()));
    }
    source = cause.source();
  }
  None
}

fn http_client(timeout: Duration) -> reqwest::Client {
  let policy = reqwest::redirect::Policy::custom(|attempt| {
    if plain_http_to_remote(attempt.url()) {
      let refused = RegistryError::Insecure(attempt.url().to_string());
      attempt.error(refused)
    } else if attempt.previous().len() >= MAX_REDIRECTS {
      attempt.error("too many redirects")
    } else {
      attempt.follow()
    }
  });
  reqwest::Client::builder()
    .connect_timeout(timeout)
    .timeout(timeout)
    .redirect(policy)
    .build()
    .expect("the reqwest client builds with static timeouts and the default TLS backend")
}

/// Which versions an upgrade or install may pick.
#[derive(Debug, Clone, Default)]
pub struct VersionRule {
  /// The version in use now; unless `cross_major` is set, only versions of its major are allowed.
  pub current: Option<Version>,
  /// Allow versions of a different major than `current`.
  pub cross_major: bool,
  /// Allow pre-release versions; they are also allowed when `current` is itself a pre-release.
  pub pre: bool,
}

impl VersionRule {
  fn allows(&self, candidate: &Version) -> bool {
    let current_is_pre = self.current.as_ref().is_some_and(|c| !c.pre.is_empty());
    if !candidate.pre.is_empty() && !self.pre && !current_is_pre {
      return false;
    }
    match &self.current {
      Some(current) if !self.cross_major => current.major == candidate.major,
      _ => true,
    }
  }
}

/// Whether a registry `host[:port]` is a loopback address: `localhost`, 127.0.0.0/8 or `[::1]`.
#[must_use]
pub fn is_loopback(registry: &str) -> bool {
  let valid_port = |port: &str| !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit());
  let host = if let Some(rest) = registry.strip_prefix('[') {
    match rest.split_once(']') {
      Some((host, "")) => host,
      Some((host, tail)) if tail.strip_prefix(':').is_some_and(valid_port) => host,
      _ => return false,
    }
  } else if registry.parse::<std::net::Ipv6Addr>().is_ok() {
    registry
  } else {
    match registry.split_once(':') {
      Some((host, port)) if valid_port(port) => host,
      Some(_) => return false,
      None => registry,
    }
  };
  host.eq_ignore_ascii_case("localhost") || host.parse::<std::net::IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

/// Anonymous client for the Moonlit registry's GraphQL API.
#[derive(Debug, Clone)]
pub struct RegistryClient {
  url: String,
  http: reqwest::Client,
}

#[derive(Deserialize)]
struct Envelope<T> {
  data: Option<T>,
  #[serde(default)]
  errors: Vec<GraphQlMessage>,
}

#[derive(Deserialize)]
struct GraphQlMessage {
  message: String,
}

#[derive(Deserialize)]
struct SearchData {
  packages: Page,
}

#[derive(Deserialize)]
struct Page {
  items: Vec<PackageHit>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VersionData {
  package_version: Option<VersionDetail>,
}

#[derive(Deserialize)]
struct VersionsData {
  package: Option<PackageVersions>,
}

#[derive(Deserialize)]
struct PackageVersions {
  versions: Vec<VersionSummary>,
}

impl RegistryClient {
  /// Builds a client for a registry host, or for an `http(s)://` URL; `/graphql` is appended unless already present.
  /// Fails with [`RegistryError::Insecure`] for an `http://` URL whose host is not loopback.
  pub fn for_host(host: &str) -> Result<Self, RegistryError> {
    if has_scheme(host, "http://") || has_scheme(host, "https://") {
      let trimmed = host.trim_end_matches('/');
      if trimmed.ends_with("/graphql") {
        Self::with_base_url(trimmed)
      } else {
        Self::with_base_url(&format!("{trimmed}/graphql"))
      }
    } else if is_loopback(host) {
      Self::with_base_url(&format!("http://{host}/graphql"))
    } else {
      Self::with_base_url(&format!("https://{host}/graphql"))
    }
  }

  /// Builds a client that posts to `url` exactly as given.
  /// Fails with [`RegistryError::Insecure`] for an `http://` URL whose host is not loopback.
  pub fn with_base_url(url: &str) -> Result<Self, RegistryError> {
    if has_scheme(url, "http://")
      && reqwest::Url::parse(url)
        .ok()
        .is_none_or(|parsed| plain_http_to_remote(&parsed))
    {
      return Err(RegistryError::Insecure(url.to_string()));
    }
    Ok(Self {
      url: url.to_string(),
      http: http_client(REGISTRY_TIMEOUT),
    })
  }

  /// Replaces the connect and request timeouts, [`REGISTRY_TIMEOUT`] each by default.
  #[must_use]
  pub fn with_timeout(self, timeout: Duration) -> Self {
    Self {
      url: self.url,
      http: http_client(timeout),
    }
  }

  /// The GraphQL endpoint this client posts to.
  #[must_use]
  pub fn base_url(&self) -> &str {
    &self.url
  }

  /// Searches packages by free text, returning at most `limit` hits.
  pub async fn search(&self, query: &str, limit: u32) -> Result<Vec<PackageHit>, RegistryError> {
    let variables = json!({ "filter": { "query": query }, "paging": { "limit": limit } });
    let data: SearchData = self.execute(SEARCH_QUERY, variables).await?;
    Ok(data.packages.items)
  }

  /// Fetches one version of a package, or `None` when it does not exist or the registry answers for another version.
  pub async fn package_version(&self, org: &str, name: &str, version: &str) -> Result<Option<VersionDetail>, RegistryError> {
    let variables = json!({ "org": org, "name": name, "version": version });
    let data: VersionData = self.execute(PACKAGE_VERSION_QUERY, variables).await?;
    Ok(data.package_version.filter(|detail| detail.version == version))
  }

  /// Lists every published version of a package, yanked ones included.
  pub async fn versions(&self, org: &str, name: &str) -> Result<Vec<VersionSummary>, RegistryError> {
    let variables = json!({ "org": org, "name": name });
    let data: VersionsData = self.execute(VERSIONS_QUERY, variables).await?;
    Ok(data.package.map(|p| p.versions).unwrap_or_default())
  }

  /// Returns the highest non-yanked version allowed by `rule` that was built for `world`, asking about ten candidates at a time.
  pub async fn newest_compatible(
    &self,
    org: &str,
    name: &str,
    world: &str,
    rule: VersionRule,
  ) -> Result<Option<VersionDetail>, RegistryError> {
    let mut candidates: Vec<(Version, String)> = self
      .versions(org, name)
      .await?
      .into_iter()
      .filter(|v| !v.yanked)
      .filter_map(|v| Version::parse(&v.version).ok().map(|parsed| (parsed, v.version)))
      .filter(|(parsed, _)| rule.allows(parsed))
      .collect();
    candidates.sort_by(|a, b| b.0.cmp(&a.0));

    for batch in candidates.chunks(MAX_CANDIDATES) {
      if let Some(detail) = self.first_for_world(org, name, world, batch).await? {
        return Ok(Some(detail));
      }
    }
    Ok(None)
  }

  async fn first_for_world(
    &self,
    org: &str,
    name: &str,
    world: &str,
    batch: &[(Version, String)],
  ) -> Result<Option<VersionDetail>, RegistryError> {
    let mut lookups = tokio::task::JoinSet::new();
    for (index, (_, raw)) in batch.iter().enumerate() {
      let (client, org, name, raw) = (self.clone(), org.to_string(), name.to_string(), raw.clone());
      lookups.spawn(async move { (index, client.package_version(&org, &name, &raw).await) });
    }
    let mut answers = Vec::new();
    while let Some(joined) = lookups.join_next().await {
      answers.push(joined.map_err(|e| RegistryError::Http(format!("version lookup failed: {e}")))?);
    }
    answers.sort_by_key(|(index, _)| *index);

    for (_, answer) in answers {
      if let Some(detail) = answer?
        && detail.world_version.as_deref() == Some(world)
      {
        return Ok(Some(detail));
      }
    }
    Ok(None)
  }

  async fn execute<T: DeserializeOwned>(&self, query: &str, variables: Value) -> Result<T, RegistryError> {
    let body = json!({ "query": query, "variables": variables }).to_string();
    let response = self
      .http
      .post(&self.url)
      .header("content-type", "application/json")
      .body(body)
      .send()
      .await
      .map_err(|e| refused_redirect(&e).unwrap_or_else(|| RegistryError::Unreachable(format!("POST {}: {e}", self.url))))?;
    let status = response.status();
    if matches!(status.as_u16(), 404 | 405) {
      return Err(RegistryError::NotARegistry(self.url.clone()));
    }
    if matches!(status.as_u16(), 401 | 403) {
      return Err(RegistryError::Denied {
        url: self.url.clone(),
        status: status.as_u16(),
      });
    }
    if !status.is_success() {
      return Err(RegistryError::Http(format!("POST {}: HTTP {status}", self.url)));
    }
    let bytes = response
      .bytes()
      .await
      .map_err(|e| RegistryError::Unreachable(format!("POST {}: {e}", self.url)))?;
    let body: Value = serde_json::from_slice(&bytes).map_err(|_| RegistryError::NotARegistry(self.url.clone()))?;
    if body.get("data").or_else(|| body.get("errors")).is_none() {
      return Err(RegistryError::NotARegistry(self.url.clone()));
    }
    let envelope: Envelope<T> = serde_json::from_value(body).map_err(|e| RegistryError::Decode(e.to_string()))?;
    if let Some(first) = envelope.errors.into_iter().next() {
      return Err(RegistryError::GraphQl(first.message));
    }
    envelope
      .data
      .ok_or_else(|| RegistryError::Decode("response has neither data nor errors".to_string()))
  }
}

#[cfg(test)]
mod tests {
  use super::{is_loopback, plain_http_to_remote};

  #[test]
  fn only_plain_http_to_a_non_loopback_host_counts_as_remote() {
    for (url, remote) in [
      ("https://registry.moonlit.rs/graphql", false),
      ("https://10.0.0.1:5000/graphql", false),
      ("http://127.0.0.1:5000/graphql", false),
      ("http://localhost/graphql", false),
      ("http://[::1]:5000/graphql", false),
      ("http://registry.moonlit.rs/graphql", true),
      ("http://10.0.0.1:5000/graphql", true),
      ("http://127.0.0.1.nip.io/graphql", true),
      ("http://[2001:db8::1]/graphql", true),
    ] {
      let parsed = reqwest::Url::parse(url).unwrap();
      assert_eq!(plain_http_to_remote(&parsed), remote, "{url}");
    }
  }

  #[test]
  fn loopback_registries_are_recognised() {
    for registry in [
      "localhost:5000",
      "127.0.0.1:8080",
      "127.0.0.1",
      "127.0.0.2:5185",
      "[::1]",
      "[::1]:5000",
      "::1",
      "LOCALHOST",
      "LOCALHOST:5185",
    ] {
      assert!(is_loopback(registry), "{registry}");
    }
  }

  #[test]
  fn remote_registries_are_not_loopback() {
    for registry in [
      "registry.moonlit.rs",
      "ghcr.io:443",
      "10.0.0.1:5000",
      "[2001:db8::1]:5000",
      "localhost.example.com",
      "127.evil.com:5000",
      "127.0.0.1.nip.io",
      "[::ffff:127.0.0.1]:5000",
      "host:port",
      "localhost:x@evil.com",
      "localhost:80@evil.com",
      "localhost:abc",
      "localhost:",
      "127.0.0.1@evil.com",
      "127.0.0.1:pw@evil.com",
      "[::1]x",
      "[::1]:",
      "[::1]:x",
      "localhost/x",
      "localhost ",
    ] {
      assert!(!is_loopback(registry), "{registry}");
    }
  }
}
