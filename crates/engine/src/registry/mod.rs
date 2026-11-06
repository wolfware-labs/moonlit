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
  /// The request could not be sent or the server answered with a non-success status.
  #[error("registry request failed: {0}")]
  Http(String),
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
}

/// How long a registry request may take to connect, and in all, before it fails.
pub const REGISTRY_TIMEOUT: Duration = Duration::from_secs(10);

fn http_client(timeout: Duration) -> reqwest::Client {
  reqwest::Client::builder()
    .connect_timeout(timeout)
    .timeout(timeout)
    .build()
    .expect("the reqwest client builds with static timeouts and the default TLS backend")
}

/// Which versions an upgrade or install may pick.
#[derive(Debug, Clone, Default)]
pub struct VersionRule {
  pub current: Option<Version>,
  pub cross_major: bool,
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
  let host = match registry.strip_prefix('[') {
    Some(rest) => rest.split_once(']').map_or(rest, |(h, _)| h),
    None => registry.split_once(':').map_or(registry, |(h, _)| h),
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
  #[must_use]
  pub fn for_host(host: &str) -> Self {
    if host.starts_with("http://") || host.starts_with("https://") {
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
  #[must_use]
  pub fn with_base_url(url: &str) -> Self {
    Self {
      url: url.to_string(),
      http: http_client(REGISTRY_TIMEOUT),
    }
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

  /// Fetches one version of a package, or `None` when it does not exist.
  pub async fn package_version(&self, org: &str, name: &str, version: &str) -> Result<Option<VersionDetail>, RegistryError> {
    let variables = json!({ "org": org, "name": name, "version": version });
    let data: VersionData = self.execute(PACKAGE_VERSION_QUERY, variables).await?;
    Ok(data.package_version)
  }

  /// Lists every published version of a package, yanked ones included.
  pub async fn versions(&self, org: &str, name: &str) -> Result<Vec<VersionSummary>, RegistryError> {
    let variables = json!({ "org": org, "name": name });
    let data: VersionsData = self.execute(VERSIONS_QUERY, variables).await?;
    Ok(data.package.map(|p| p.versions).unwrap_or_default())
  }

  /// Returns the highest non-yanked version allowed by `rule` that was built for `world`.
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

    for (_, raw) in candidates {
      if let Some(detail) = self.package_version(org, name, &raw).await?
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
      .map_err(|e| RegistryError::Http(format!("POST {}: {e}", self.url)))?;
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
      .map_err(|e| RegistryError::Http(format!("POST {}: {e}", self.url)))?;
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
  use super::is_loopback;

  #[test]
  fn loopback_registries_are_recognised() {
    for registry in ["localhost:5000", "127.0.0.1:8080", "127.0.0.1", "[::1]:5000", "LOCALHOST"] {
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
    ] {
      assert!(!is_loopback(registry), "{registry}");
    }
  }
}
