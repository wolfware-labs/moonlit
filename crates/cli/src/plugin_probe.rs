use std::sync::Arc;

use crate::display::display_safe_text;
use moonlit_engine::cache::Cache;
use moonlit_engine::engine::Engine;
use moonlit_engine::logging::LogLevel;
use moonlit_engine::pipeline::config::Permissions;
use moonlit_engine::plugin::host::HostEventSink;
use moonlit_engine::plugin::middleware::MiddlewareInfo;
use moonlit_engine::plugin::resolver::{PluginSource, ResolveError, ResolveOptions, resolve};
use moonlit_engine::plugin::world::{self, WorldError, WorldMismatch};
use moonlit_engine::plugin::{Plugin, PluginInstanceConfig, PluginMetadata};

/// What a plugin component says about itself.
#[derive(Debug)]
pub struct Probe {
  pub meta: PluginMetadata,
  pub middlewares: Vec<MiddlewareInfo>,
}

/// Why a plugin could not be probed.
#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
  /// The URL could not be resolved to a component through the cache.
  #[error("cannot resolve '{url}': {message}")]
  Resolve { url: String, message: String, network: bool },
  /// The component targets a plugin contract this CLI does not support.
  #[error("'{url}': {mismatch}")]
  Incompatible { url: String, mismatch: WorldMismatch },
  /// The component could not be read or instantiated.
  #[error("cannot load '{url}': {message}")]
  Load { url: String, message: String },
}

/// Resolves a plugin URL through the cache and asks the component for its metadata and middlewares.
pub async fn probe(url: &str) -> Result<Probe, ProbeError> {
  let resolve_error = |message: String, network: bool| ProbeError::Resolve {
    url: url.to_string(),
    message,
    network,
  };
  let load_error = |message: String| ProbeError::Load {
    url: url.to_string(),
    message,
  };
  let source = url.parse::<PluginSource>().map_err(|e| resolve_error(e.to_string(), false))?;
  let cache = Cache::new().map_err(|e| resolve_error(e.to_string(), false))?;
  let resolved = resolve(&source, &ResolveOptions::default(), &cache, None)
    .await
    .map_err(|e| resolve_error(e.to_string(), matches!(e, ResolveError::Network(_))))?;
  let bytes = std::fs::read(&resolved.wasm_path).map_err(|e| load_error(e.to_string()))?;
  world::check(&bytes).map_err(|e| match e {
    WorldError::Mismatch(mismatch) => ProbeError::Incompatible {
      url: url.to_string(),
      mismatch,
    },
    WorldError::Load(e) => load_error(e.to_string()),
  })?;
  let (meta, middlewares) = introspect(&bytes).await.map_err(|e| load_error(e.to_string()))?;
  Ok(Probe { meta, middlewares })
}

struct SilentSink;
impl HostEventSink for SilentSink {
  fn log(&self, _step: &str, _level: LogLevel, _message: &str) {}
  fn progress(&self, _step: &str, _message: &str) {}
}

/// Instantiates a component with no permissions and reads its metadata and middlewares.
/// The error text comes back escaped for the terminal.
pub async fn introspect(bytes: &[u8]) -> anyhow::Result<(PluginMetadata, Vec<MiddlewareInfo>)> {
  read_plugin(bytes).await.map_err(|e| for_terminal(&e))
}

fn for_terminal(error: &anyhow::Error) -> anyhow::Error {
  anyhow::anyhow!(display_safe_text(&error.to_string()))
}

async fn read_plugin(bytes: &[u8]) -> anyhow::Result<(PluginMetadata, Vec<MiddlewareInfo>)> {
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn an_introspection_error_is_escaped_for_the_terminal() {
    let shown = for_terminal(&anyhow::anyhow!("blocked from running 'git\u{1b}[2K'\nat list")).to_string();
    assert_eq!(shown, "blocked from running 'git\\x1b[2K'\nat list");
  }
}
