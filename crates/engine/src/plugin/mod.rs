mod instance;
mod model;
mod publish;
mod resolver;

use crate::engine::Engine;
use crate::host::HostEventSink;
use crate::host::state::HostState;
use crate::host::wit::PluginHost;
use crate::logging::LogLevel;
use crate::plugin::instance::PluginInstance;
pub use crate::plugin::model::{MiddlewareInfo, PluginMetadata};
pub use crate::plugin::publish::{PublishMeta, new_push_client, publish_plugin};
pub use crate::plugin::resolver::PluginSource;
use std::sync::Arc;
use wasmtime::Store;
use wasmtime::component::Component;

pub struct Plugin {
    metadata: PluginMetadata,
}

pub enum PluginError {
    Compile(String),
    Instantiate(String),
}

impl Plugin {
    pub async fn instantiate(
        engine: &Engine,
        component_bytes: &[u8],
        cfg: InstanceConfig,
        events: Arc<dyn HostEventSink>,
    ) -> Result<PluginInstance, PluginError> {
        let wasi =
            auth::build_wasi_ctx(&cfg).map_err(|e| PluginError::Instantiate(e.to_string()))?;
        events.log(
            "",
            LogLevel::Debug,
            &format!(
                "plugin grants — network={:?} exec={:?} env={:?} filesystem={:?}",
                cfg.permissions.network,
                cfg.permissions.exec,
                cfg.permissions.env,
                cfg.permissions.filesystem
            ),
        );
        let state = HostState::new(
            wasi,
            AllowlistHooks::new(&cfg.permissions, events.clone()),
            events,
            cfg.config_view,
            perms::exec_globset(&cfg.permissions.exec),
        );
        let mut store = Store::new(engine, state);
        let component = Component::from_binary(engine, component_bytes)
            .map_err(|e| PluginError::Load(e.to_string()))?;
        let bindings = PluginHost::instantiate_async(&mut store, &component, &linker)
            .await
            .map_err(|e| PluginError::Instantiate(e.to_string()))?;
        Ok(PluginInstance { store, bindings })
    }
}
