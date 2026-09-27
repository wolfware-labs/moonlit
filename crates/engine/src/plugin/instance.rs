use crate::pipeline::config::Permissions;
use crate::plugin::host::ReleaseContext;
use crate::plugin::host::state::HostState;
use crate::plugin::middleware::{MiddlewareInfo, MiddlewareResult};
use crate::plugin::wit::PluginHost;
use crate::plugin::{PluginError, PluginMetadata};
use std::path::PathBuf;
use wasmtime::Store;

pub struct PluginInstance {
  store: Store<HostState>,
  bindings: PluginHost,
}

pub struct PluginInstanceConfig {
  pub working_directory: PathBuf,
  pub permissions: Permissions,
  pub config_view: serde_json::Value,
  pub env_snapshot: Vec<(String, String)>,
}

impl PluginInstance {
  pub fn new(store: Store<HostState>, bindings: PluginHost) -> Self {
    Self { store, bindings }
  }

  pub async fn describe(&mut self) -> Result<PluginMetadata, PluginError> {
    match self.bindings.call_describe(&mut self.store).await {
      Ok(meta) => Ok(convert::metadata(meta)),
      Err(e) => Err(PluginError::Trap {
        op: "describe".to_string(),
        message: format!("{e:?}"),
      }),
    }
  }

  pub async fn init(&mut self, plugin_config: &serde_json::Value) -> Result<PluginMetadata, String> {
    self.store.data_mut().current_step = "init".to_string();
    let json = plugin_config.to_string();
    match self.bindings.call_init(&mut self.store, &json).await {
      Ok(Ok(meta)) => Ok(convert::metadata(meta)),
      Ok(Err(msg)) => Err(msg),
      Err(trap) => Err(format!("plugin trapped during init: {trap:?}")),
    }
  }

  pub async fn execute(
    &mut self,
    middleware: &str,
    ctx: ReleaseContext,
    config: &serde_json::Value,
  ) -> Result<MiddlewareResult, PluginError> {
    self.store.data_mut().current_step = ctx.step_name.clone();
    let raw_ctx = convert::release_context_to_raw(&ctx);
    let json = config.to_string();
    match self.bindings.call_execute(&mut self.store, middleware, &raw_ctx, &json).await {
      Ok(raw) => convert::middleware_result(raw),
      Err(e) => Err(PluginError::Trap {
        op: format!("execute {middleware}"),
        message: format!("{e:?}"),
      }),
    }
  }

  pub async fn list_middlewares(&mut self) -> Result<Vec<MiddlewareInfo>, PluginError> {
    match self.bindings.call_list_middlewares(&mut self.store).await {
      Ok(list) => Ok(list.into_iter().map(convert::middleware_info).collect()),
      Err(e) => Err(PluginError::Trap {
        op: "list-middlewares".to_string(),
        message: format!("{e:?}"),
      }),
    }
  }
}
