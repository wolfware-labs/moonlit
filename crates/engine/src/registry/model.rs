use serde::Deserialize;

/// Verification state the registry reports for a package or version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VerificationStatus {
  Pending,
  Verified,
  Unverified,
  Failed,
}

/// One package returned by a registry search.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageHit {
  #[serde(rename = "orgSlug")]
  pub org: String,
  pub name: String,
  pub description: Option<String>,
  pub latest_version: Option<String>,
  pub status: VerificationStatus,
  #[serde(default)]
  pub middlewares: Vec<String>,
}

/// A middleware a package version exposes.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct MiddlewareSummary {
  pub name: String,
  pub description: Option<String>,
}

/// The details of one published package version.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionDetail {
  pub version: String,
  pub world_version: Option<String>,
  pub status: VerificationStatus,
  #[serde(default)]
  pub middlewares: Vec<MiddlewareSummary>,
}

/// One entry in a package's version list.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct VersionSummary {
  pub version: String,
  pub yanked: bool,
  pub status: VerificationStatus,
}
