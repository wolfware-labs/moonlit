mod cache;
mod file;
mod http;
mod model;
mod oci;

use crate::plugin::resolver::cache::Cache;
pub use crate::plugin::resolver::model::{
    PluginSource, ProgressFn, ResolveError, ResolveOptions, ResolvedPlugin,
};

//

//



// trait Resolver {
//     resolve(path: & str) -> Result<ResolvedPlugin, ResolveError>;
// }

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
