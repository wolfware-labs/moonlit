use crate::pipeline::config::Permissions;
use crate::plugin::host::ReleaseContext;
use crate::plugin::host::state::Denial;
use crate::plugin::host::state::HostState;
use crate::plugin::middleware::{MiddlewareInfo, MiddlewareResult};
use crate::plugin::permissions::{DeclaredPermission, optional_for_target};
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
    let _ = self.store.data().take_denials();
    let json = plugin_config.to_string();
    let result = match self.bindings.call_init(&mut self.store, &json).await {
      Ok(Ok(meta)) => Ok(meta.into()),
      Ok(Err(msg)) => Err(msg),
      Err(trap) => Err(format!("plugin trapped during init: {trap:?}")),
    };
    match self.denied() {
      None => result,
      Some(denied) => Err(match result {
        Err(own) => format!("{own}; {denied}"),
        Ok(_) => denied,
      }),
    }
  }

  pub async fn execute(
    &mut self,
    middleware: &str,
    ctx: ReleaseContext,
    config: &serde_json::Value,
  ) -> Result<MiddlewareResult, PluginError> {
    self.store.data_mut().set_step(&ctx.step_name);
    let _ = self.store.data().take_denials();
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
    let denials = self.store.data().take_denials();
    apply_denials(result, &denials, self.store.data().declared())
  }

  pub async fn list_middlewares(&mut self) -> Result<Vec<MiddlewareInfo>, PluginError> {
    let _ = self.store.data().take_denials();
    let result = self.bindings.call_list_middlewares(&mut self.store).await;
    let message = match (result, self.denied()) {
      (Ok(list), None) => return Ok(list.into_iter().map(std::convert::Into::into).collect()),
      (Ok(_), Some(denied)) => denied,
      (Err(e), None) => format!("{e:?}"),
      (Err(e), Some(denied)) => format!("{e:?}; {denied}"),
    };
    Err(PluginError::Trap {
      op: "list-middlewares".to_string(),
      message,
    })
  }

  fn denied(&self) -> Option<String> {
    let denials = self.store.data().take_denials();
    (!denials.is_empty()).then(|| denial_message(&denials, self.store.data().declared()))
  }
}

fn apply_denials(
  result: Result<MiddlewareResult, PluginError>,
  denials: &[Denial],
  declared: &[DeclaredPermission],
) -> Result<MiddlewareResult, PluginError> {
  if denials.is_empty() {
    return result;
  }
  let denied = denial_message(denials, declared);
  match result {
    Ok(mut r) => {
      r.successful = false;
      r.error_message = Some(match r.error_message.take() {
        Some(own) if !own.is_empty() => format!("{own}; {denied}"),
        _ => denied,
      });
      Ok(r)
    }
    Err(PluginError::Trap { op, message }) => Err(PluginError::Trap {
      op,
      message: format!("{message}; {denied}"),
    }),
    Err(other) => Ok(MiddlewareResult {
      successful: false,
      error_message: Some(format!("{other}; {denied}")),
      warnings: vec![],
      output: vec![],
    }),
  }
}

fn denial_message(denials: &[Denial], declared: &[DeclaredPermission]) -> String {
  denials
    .iter()
    .map(|d| match optional_for_target(declared, d.kind, &d.target) {
      Some(p) => format!("{d} (optional permission: \"{}\")", p.reason),
      None => d.to_string(),
    })
    .collect::<Vec<_>>()
    .join("; ")
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::plugin::permissions::PermissionKind;

  fn denial(kind: PermissionKind, target: &str) -> Denial {
    Denial {
      kind,
      target: target.to_string(),
    }
  }

  fn declared(kind: PermissionKind, values: &[&str], reason: &str, optional: bool) -> DeclaredPermission {
    DeclaredPermission {
      kind,
      values: values.iter().map(|v| (*v).to_string()).collect(),
      reason: reason.to_string(),
      optional,
    }
  }

  fn result(successful: bool, error: Option<&str>) -> MiddlewareResult {
    MiddlewareResult {
      successful,
      error_message: error.map(str::to_string),
      warnings: vec![],
      output: vec![],
    }
  }

  #[test]
  fn exec_denial_names_the_program_and_the_exec_field() {
    assert_eq!(
      denial_message(&[denial(PermissionKind::Exec, "git")], &[]),
      "blocked from running 'git' \u{2014} add it to permissions.exec"
    );
  }

  #[test]
  fn network_denial_names_the_host_and_the_network_field() {
    assert_eq!(
      denial_message(&[denial(PermissionKind::Network, "api.example.com")], &[]),
      "blocked from connecting to 'api.example.com' \u{2014} add it to permissions.network"
    );
  }

  #[test]
  fn network_denial_matching_an_optional_permission_quotes_its_reason() {
    let declared = [declared(PermissionKind::Network, &["*.example.com"], "Calls the API.", true)];
    assert_eq!(
      denial_message(&[denial(PermissionKind::Network, "api.example.com")], &declared),
      "blocked from connecting to 'api.example.com' \u{2014} add it to permissions.network (optional permission: \"Calls the API.\")"
    );
  }

  #[test]
  fn denial_of_another_kind_still_prints_the_kind() {
    let message = denial_message(&[denial(PermissionKind::Env, "HOME")], &[]);
    assert_eq!(message, "blocked from using 'HOME' \u{2014} add it to permissions.env");
  }

  #[test]
  fn several_denials_are_joined() {
    let message = denial_message(
      &[
        denial(PermissionKind::Exec, "git"),
        denial(PermissionKind::Network, "a.example.com"),
      ],
      &[],
    );
    assert!(
      message.contains("running 'git'") && message.contains("; blocked from connecting to 'a.example.com'"),
      "{message}"
    );
  }

  #[test]
  fn a_denial_fails_a_successful_result() {
    let out = apply_denials(Ok(result(true, None)), &[denial(PermissionKind::Exec, "git")], &[]).unwrap();
    assert!(!out.successful);
    assert_eq!(
      out.error_message.as_deref(),
      Some("blocked from running 'git' \u{2014} add it to permissions.exec")
    );
  }

  #[test]
  fn a_denial_is_appended_to_the_plugins_own_error() {
    let out = apply_denials(
      Ok(result(false, Some("git tag failed"))),
      &[denial(PermissionKind::Exec, "git")],
      &[],
    )
    .unwrap();
    assert!(!out.successful);
    assert_eq!(
      out.error_message.as_deref(),
      Some("git tag failed; blocked from running 'git' \u{2014} add it to permissions.exec")
    );
  }

  #[test]
  fn a_denial_is_appended_to_a_trap() {
    let trap = PluginError::Trap {
      op: "execute tag".to_string(),
      message: "unreachable".to_string(),
    };
    let Err(PluginError::Trap { op, message }) = apply_denials(Err(trap), &[denial(PermissionKind::Exec, "git")], &[]) else {
      panic!("expected a trap");
    };
    assert_eq!(op, "execute tag");
    assert_eq!(
      message,
      "unreachable; blocked from running 'git' \u{2014} add it to permissions.exec"
    );
  }

  #[test]
  fn an_empty_own_error_is_replaced_by_the_denial() {
    let out = apply_denials(Ok(result(false, Some(""))), &[denial(PermissionKind::Exec, "git")], &[]).unwrap();
    assert_eq!(
      out.error_message.as_deref(),
      Some("blocked from running 'git' \u{2014} add it to permissions.exec")
    );
  }

  #[test]
  fn a_denial_is_kept_next_to_a_decode_failure() {
    let source = serde_json::from_str::<serde_json::Value>("{").unwrap_err();
    let bad = PluginError::BadJson {
      context: "output".to_string(),
      source,
    };
    let out = apply_denials(Err(bad), &[denial(PermissionKind::Exec, "git")], &[]).unwrap();
    assert!(!out.successful);
    let message = out.error_message.expect("message");
    assert!(message.contains("malformed JSON for output"), "{message}");
    assert!(
      message.ends_with("; blocked from running 'git' \u{2014} add it to permissions.exec"),
      "{message}"
    );
  }

  #[test]
  fn escape_characters_in_a_denied_target_are_escaped_for_the_terminal() {
    let message = denial_message(&[denial(PermissionKind::Exec, "git\u{1b}[2K")], &[]);
    let shown = crate::sanitize::display_safe_text(&message);
    assert!(!shown.contains('\u{1b}'), "{shown:?}");
    assert!(shown.contains("git\\x1b[2K"), "{shown}");
  }

  #[test]
  fn without_denials_the_result_is_untouched() {
    let out = apply_denials(Ok(result(false, Some("boom"))), &[], &[]).unwrap();
    assert_eq!(out, result(false, Some("boom")));
    let Err(PluginError::Load(message)) = apply_denials(Err(PluginError::Load("x".to_string())), &[], &[]) else {
      panic!("expected the same error back");
    };
    assert_eq!(message, "x");
  }
}
