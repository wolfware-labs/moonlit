use crate::pipeline::config::Permissions;
use crate::plugin::host::ReleaseContext;
use crate::plugin::host::state::Denial;
use crate::plugin::host::state::HostState;
use crate::plugin::middleware::{MiddlewareInfo, MiddlewareResult};
use crate::plugin::permissions::{DeclaredPermission, PermissionKind, optional_for_target};
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
  #[must_use]
  pub fn new(store: Store<HostState>, bindings: PluginHost) -> Self {
    Self { store, bindings }
  }

  pub fn set_declared(&mut self, declared: Vec<DeclaredPermission>) {
    self.store.data_mut().set_declared(declared);
  }

  #[must_use]
  pub fn declared(&self) -> &[DeclaredPermission] {
    self.store.data().declared()
  }

  pub async fn describe(&mut self) -> Result<PluginMetadata, PluginError> {
    match self.bindings.call_describe(&mut self.store).await {
      Ok(meta) => Ok(meta.into()),
      Err(e) => Err(PluginError::Trap {
        op: "describe".to_string(),
        message: format!("{e:?}"),
      }),
    }
  }

  pub async fn init(&mut self, plugin_config: &serde_json::Value) -> Result<PluginMetadata, String> {
    self.store.data_mut().set_step("init");
    let json = plugin_config.to_string();
    match self.bindings.call_init(&mut self.store, &json).await {
      Ok(Ok(meta)) => Ok(meta.into()),
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
    self.store.data_mut().set_step(&ctx.step_name);
    self.store.data().denials().lock().unwrap().clear();
    let json = config.to_string();
    let result = match self
      .bindings
      .call_execute(&mut self.store, middleware, &ctx.into(), &json)
      .await
    {
      Ok(raw) => raw.into(),
      Err(e) => Err(PluginError::Trap {
        op: format!("execute {middleware}"),
        message: format!("{e:?}"),
      }),
    };
    let denials = std::mem::take(&mut *self.store.data().denials().lock().unwrap());
    match result {
      Ok(mut r) if !denials.is_empty() => {
        r.successful = false;
        r.error_message = Some(denial_message(&denials, self.store.data().declared()));
        Ok(r)
      }
      other => other,
    }
  }

  pub async fn list_middlewares(&mut self) -> Result<Vec<MiddlewareInfo>, PluginError> {
    match self.bindings.call_list_middlewares(&mut self.store).await {
      Ok(list) => Ok(list.into_iter().map(std::convert::Into::into).collect()),
      Err(e) => Err(PluginError::Trap {
        op: "list-middlewares".to_string(),
        message: format!("{e:?}"),
      }),
    }
  }
}

fn denial_message(denials: &[Denial], declared: &[DeclaredPermission]) -> String {
  denials
    .iter()
    .map(|d| {
      let (verb, field) = match d.kind {
        PermissionKind::Network => ("connecting to", "network"),
        _ => ("running", "exec"),
      };
      let mut msg = format!("blocked from {verb} '{}' — add it to permissions.{field}", d.target);
      if let Some(p) = optional_for_target(declared, d.kind, &d.target) {
        msg = format!("{msg} (optional permission: \"{}\")", p.reason);
      }
      msg
    })
    .collect::<Vec<_>>()
    .join("; ")
}
