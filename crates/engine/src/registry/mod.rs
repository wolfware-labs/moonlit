mod model;

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

#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
  #[error("registry request failed: {0}")]
  Http(String),
  #[error("registry returned an error: {0}")]
  GraphQl(String),
  #[error("registry response could not be decoded: {0}")]
  Decode(String),
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
  /// Builds a client for a registry host, or for a full `http(s)://` base URL.
  #[must_use]
  pub fn for_host(host: &str) -> Self {
    if host.starts_with("http://") || host.starts_with("https://") {
      Self::with_base_url(host)
    } else {
      Self::with_base_url(&format!("https://{host}/graphql"))
    }
  }

  #[must_use]
  pub fn with_base_url(url: &str) -> Self {
    Self {
      url: url.to_string(),
      http: reqwest::Client::new(),
    }
  }

  #[must_use]
  pub fn base_url(&self) -> &str {
    &self.url
  }

  pub async fn search(&self, query: &str, limit: u32) -> Result<Vec<PackageHit>, RegistryError> {
    let variables = json!({ "filter": { "query": query }, "paging": { "limit": limit } });
    let data: SearchData = self.execute(SEARCH_QUERY, variables).await?;
    Ok(data.packages.items)
  }

  pub async fn package_version(&self, org: &str, name: &str, version: &str) -> Result<Option<VersionDetail>, RegistryError> {
    let variables = json!({ "org": org, "name": name, "version": version });
    let data: VersionData = self.execute(PACKAGE_VERSION_QUERY, variables).await?;
    Ok(data.package_version)
  }

  pub async fn versions(&self, org: &str, name: &str) -> Result<Vec<VersionSummary>, RegistryError> {
    let variables = json!({ "org": org, "name": name });
    let data: VersionsData = self.execute(VERSIONS_QUERY, variables).await?;
    Ok(data.package.map(|p| p.versions).unwrap_or_default())
  }

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
    if !status.is_success() {
      return Err(RegistryError::Http(format!("POST {}: HTTP {status}", self.url)));
    }
    let bytes = response
      .bytes()
      .await
      .map_err(|e| RegistryError::Http(format!("POST {}: {e}", self.url)))?;
    let envelope: Envelope<T> = serde_json::from_slice(&bytes).map_err(|e| RegistryError::Decode(e.to_string()))?;
    if let Some(first) = envelope.errors.into_iter().next() {
      return Err(RegistryError::GraphQl(first.message));
    }
    envelope
      .data
      .ok_or_else(|| RegistryError::Decode("response has neither data nor errors".to_string()))
  }
}
