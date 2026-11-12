mod file;
mod http;
mod model;
pub mod oci;

use std::time::Duration;

use crate::cache::Cache;
pub use crate::plugin::resolver::model::{PluginSource, ProgressFn, ResolveError, ResolveOptions, ResolvedPlugin};

pub(crate) const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
pub(crate) const READ_TIMEOUT: Duration = Duration::from_secs(30);

pub async fn resolve(
  source: &PluginSource,
  opts: &ResolveOptions,
  cache: &Cache,
  progress: Option<ProgressFn<'_>>,
) -> Result<ResolvedPlugin, ResolveError> {
  match source {
    PluginSource::File(path) => file::resolve_file(path),
    PluginSource::Http(url) => http::resolve_http(url, opts, cache, progress).await,
    PluginSource::Oci(raw_ref) => oci::resolve_oci(raw_ref, opts, cache, progress).await,
  }
}
