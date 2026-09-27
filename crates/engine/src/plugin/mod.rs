mod error;
mod instance;
pub mod middleware;
mod publish;
mod resolver;
pub mod wit;
// mod expr;
pub mod host;

use crate::engine::Engine;
use crate::logging::LogLevel;
use crate::plugin::error::PluginError;
use crate::plugin::host::{AllowlistHooks, build_wasi_ctx, exec_globset};
use crate::plugin::instance::{PluginInstance, PluginInstanceConfig};
pub use crate::plugin::publish::{PublishMeta, new_push_client, publish_plugin};
pub use crate::plugin::resolver::PluginSource;
use crate::plugin::wit::PluginHost;
use host::HostEventSink;
use host::state::HostState;
use std::sync::Arc;
use wasmtime::component::HasSelf;

#[derive(Clone, Debug, PartialEq)]
pub struct PluginMetadata {
  pub name: String,
  pub version: String,
  pub description: String,
  pub icon: Option<String>,
}

pub struct Plugin {
  metadata: PluginMetadata,
}

impl Plugin {
  pub async fn instantiate(
    engine: &Engine,
    component_bytes: &[u8],
    cfg: PluginInstanceConfig,
    events: Arc<dyn HostEventSink>,
  ) -> Result<PluginInstance, PluginError> {
    let wasi = build_wasi_ctx(&cfg).map_err(|e| PluginError::Instantiate(e.to_string()))?;
    events.log(
      "",
      LogLevel::Debug,
      &format!(
        "plugin grants — network={:?} exec={:?} env={:?} filesystem={:?}",
        cfg.permissions.network, cfg.permissions.exec, cfg.permissions.env, cfg.permissions.filesystem
      ),
    );

    let mut store = engine.build_store(HostState::new(
      wasi,
      AllowlistHooks::new(&cfg.permissions, events.clone()),
      events,
      cfg.config_view,
      exec_globset(&cfg.permissions.exec),
    ));

    let component = engine.load_component(component_bytes)?;

    let mut linker = engine.build_linker()?;

    wit::moonlit::plugin::host::add_to_linker::<_, HasSelf<_>>(&mut linker, |s| s)?;
    wit::moonlit::plugin::process::add_to_linker::<_, HasSelf<_>>(&mut linker, |s| s)?;

    let bindings = PluginHost::instantiate_async(&mut store, &component, &linker)
      .await
      .map_err(|e| crate::plugin::error::PluginError::Instantiate(e.to_string()))?;

    Ok(PluginInstance::new(store, bindings))
  }
}
