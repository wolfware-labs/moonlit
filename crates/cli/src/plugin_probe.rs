use std::sync::Arc;

use moonlit_engine::cache::Cache;
use moonlit_engine::engine::Engine;
use moonlit_engine::logging::LogLevel;
use moonlit_engine::pipeline::config::Permissions;
use moonlit_engine::plugin::host::HostEventSink;
use moonlit_engine::plugin::middleware::MiddlewareInfo;
use moonlit_engine::plugin::publish::PLUGIN_WORLD;
use moonlit_engine::plugin::resolver::{PluginSource, ResolveOptions, resolve};
use moonlit_engine::plugin::world::component_world;
use moonlit_engine::plugin::{Plugin, PluginInstanceConfig, PluginMetadata};

/// What a plugin component says about itself.
#[derive(Debug)]
pub struct Probe {
  pub meta: PluginMetadata,
  pub middlewares: Vec<MiddlewareInfo>,
  pub world: Option<String>,
}

/// Why a plugin could not be probed.
#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
  /// The URL could not be resolved to a component through the cache.
  #[error("cannot resolve '{url}': {message}")]
  Resolve { url: String, message: String },
  /// The component targets a world this CLI does not support.
  #[error("'{url}' targets {found}; this CLI supports {PLUGIN_WORLD}")]
  Incompatible { url: String, found: String },
  /// The component could not be read or instantiated.
  #[error("cannot load '{url}': {message}")]
  Load { url: String, message: String },
}

/// Resolves a plugin URL through the cache and asks the component for its metadata and middlewares.
pub async fn probe(url: &str) -> Result<Probe, ProbeError> {
  let resolve_error = |message: String| ProbeError::Resolve {
    url: url.to_string(),
    message,
  };
  let load_error = |message: String| ProbeError::Load {
    url: url.to_string(),
    message,
  };
  let source = url.parse::<PluginSource>().map_err(|e| resolve_error(e.to_string()))?;
  let cache = Cache::new().map_err(|e| resolve_error(e.to_string()))?;
  let resolved = resolve(&source, &ResolveOptions::default(), &cache, None)
    .await
    .map_err(|e| resolve_error(e.to_string()))?;
  let bytes = std::fs::read(&resolved.wasm_path).map_err(|e| load_error(e.to_string()))?;
  let world = component_world(&bytes).map_err(|e| load_error(e.to_string()))?;
  if let Some(found) = world.as_deref().filter(|w| *w != PLUGIN_WORLD) {
    return Err(ProbeError::Incompatible {
      url: url.to_string(),
      found: found.to_string(),
    });
  }
  let (meta, middlewares) = introspect(&bytes).await.map_err(|e| load_error(e.to_string()))?;
  Ok(Probe {
    meta,
    middlewares,
    world,
  })
}

struct SilentSink;
impl HostEventSink for SilentSink {
  fn log(&self, _step: &str, _level: LogLevel, _message: &str) {}
  fn progress(&self, _step: &str, _message: &str) {}
}

/// Instantiates a component with no permissions and reads its metadata and middlewares.
pub async fn introspect(bytes: &[u8]) -> anyhow::Result<(PluginMetadata, Vec<MiddlewareInfo>)> {
  let engine = Engine::try_default()?;
  let cfg = PluginInstanceConfig {
    working_directory: std::env::temp_dir(),
    permissions: Permissions::deny(),
    config_view: serde_json::json!({}),
    env_snapshot: vec![],
  };
  let mut plugin_instance = Plugin::instantiate(&engine, bytes, cfg, Arc::new(SilentSink)).await?;
  let meta = plugin_instance.describe().await?;
  let mws = plugin_instance.list_middlewares().await?;
  Ok((meta, mws))
}
