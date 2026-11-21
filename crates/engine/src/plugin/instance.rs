use crate::pipeline::config::Permissions;
use crate::plugin::host::ReleaseContext;
use crate::plugin::host::state::Denial;
use crate::plugin::host::state::HostState;
use crate::plugin::middleware::{MiddlewareInfo, MiddlewareResult};
use crate::plugin::permissions::{DeclaredPermission, invalid_declaration, optional_for_target};
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
    let meta: PluginMetadata = match self.bindings.call_describe(&mut self.store).await {
      Ok(meta) => meta.into(),
      Err(e) => {
        return Err(PluginError::Trap {
          op: "describe".to_string(),
          message: format!("{e:?}"),
        });
      }
    };
    match invalid_declaration(&meta.permissions) {
      None => Ok(meta),
      Some(why) => Err(PluginError::Load(why)),
    }
  }

  pub async fn init(&mut self, plugin_config: &serde_json::Value) -> Result<(), String> {
    self.store.data_mut().set_step("init");
    let _ = self.store.data().take_denials();
    let json = plugin_config.to_string();
    let result = match self.bindings.call_init(&mut self.store, &json).await {
      Ok(Ok(())) => Ok(()),
      Ok(Err(msg)) => Err(msg),
      Err(trap) => Err(format!("plugin trapped during init: {trap:?}")),
    };
    match self.denied() {
      None => result,
      Some(denied) => Err(match result {
        Err(own) => format!("{own}; {denied}"),
        Ok(()) => denied,
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

  /// The message for the denials that fail a call; denials of optional permissions are left out.
  fn denied(&self) -> Option<String> {
    let denials = self.store.data().take_denials();
    let (hard, _) = split_denials(&denials, self.store.data().declared());
    (!hard.is_empty()).then(|| denial_message(&hard, self.store.data().declared()))
  }
}

/// Splits denials into those that fail the call and those of permissions the plugin declared `optional`, which only
/// warn. The soft ones come back once per kind and target.
fn split_denials(denials: &[Denial], declared: &[DeclaredPermission]) -> (Vec<Denial>, Vec<Denial>) {
  let mut hard = Vec::new();
  let mut soft: Vec<Denial> = Vec::new();
  for d in denials {
    if optional_for_target(declared, d.kind, &d.target).is_none() {
      hard.push(d.clone());
    } else if !soft.contains(d) {
      soft.push(d.clone());
    }
  }
  (hard, soft)
}

fn apply_denials(
  result: Result<MiddlewareResult, PluginError>,
  denials: &[Denial],
  declared: &[DeclaredPermission],
) -> Result<MiddlewareResult, PluginError> {
  let (hard, soft) = split_denials(denials, declared);
  let warnings: Vec<String> = soft
    .iter()
    .map(|d| denial_message(std::slice::from_ref(d), declared))
    .collect();
  if hard.is_empty() {
    return result.map(|mut r| {
      r.warnings.extend(warnings);
      r
    });
  }
  let denied = denial_message(&hard, declared);
  match result {
    Ok(mut r) => {
      r.successful = false;
      r.error_message = Some(match r.error_message.take() {
        Some(own) if !own.is_empty() => format!("{own}; {denied}"),
        _ => denied,
      });
      r.warnings.extend(warnings);
      Ok(r)
    }
    Err(PluginError::Trap { op, message }) => Err(PluginError::Trap {
      op,
      message: format!("{message}; {denied}"),
    }),
    Err(other) => Ok(MiddlewareResult {
      successful: false,
      error_message: Some(format!("{other}; {denied}")),
      warnings,
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

  fn optional_git() -> Vec<DeclaredPermission> {
    vec![declared(PermissionKind::Exec, &["git"], "Reads the tag.", true)]
  }

  #[test]
  fn an_optional_denial_warns_and_leaves_the_result_alone() {
    let out = apply_denials(
      Ok(result(true, None)),
      &[denial(PermissionKind::Exec, "git")],
      &optional_git(),
    )
    .unwrap();
    assert!(out.successful);
    assert!(out.error_message.is_none());
    assert_eq!(
      out.warnings,
      ["blocked from running 'git' \u{2014} add it to permissions.exec (optional permission: \"Reads the tag.\")"]
    );
  }

  #[test]
  fn an_optional_denial_repeated_gives_one_warning() {
    let git = denial(PermissionKind::Exec, "git");
    let out = apply_denials(Ok(result(true, None)), &[git.clone(), git], &optional_git()).unwrap();
    assert_eq!(out.warnings.len(), 1, "{:?}", out.warnings);
  }

  #[test]
  fn a_required_declaration_does_not_make_a_denial_optional() {
    let required = [declared(PermissionKind::Exec, &["git"], "Reads the tag.", false)];
    let out = apply_denials(Ok(result(true, None)), &[denial(PermissionKind::Exec, "git")], &required).unwrap();
    assert!(!out.successful);
    assert!(out.warnings.is_empty());
  }

  #[test]
  fn a_mixed_call_fails_on_the_hard_denial_and_warns_about_the_soft_one() {
    let out = apply_denials(
      Ok(result(true, None)),
      &[denial(PermissionKind::Exec, "git"), denial(PermissionKind::Exec, "curl")],
      &optional_git(),
    )
    .unwrap();
    assert!(!out.successful);
    assert_eq!(
      out.error_message.as_deref(),
      Some("blocked from running 'curl' \u{2014} add it to permissions.exec")
    );
    assert_eq!(out.warnings.len(), 1, "{:?}", out.warnings);
    assert!(out.warnings[0].contains("'git'"), "{:?}", out.warnings);
  }

  #[test]
  fn an_optional_denial_next_to_a_plugin_error_keeps_the_error_and_the_message_clean() {
    let out = apply_denials(
      Ok(result(false, Some("git tag failed"))),
      &[denial(PermissionKind::Exec, "git")],
      &optional_git(),
    )
    .unwrap();
    assert!(!out.successful);
    assert_eq!(out.error_message.as_deref(), Some("git tag failed"));
    assert_eq!(out.warnings.len(), 1);
  }

  #[test]
  fn an_optional_denial_does_not_turn_a_decode_failure_into_a_result() {
    let Err(PluginError::Load(message)) = apply_denials(
      Err(PluginError::Load("x".to_string())),
      &[denial(PermissionKind::Exec, "git")],
      &optional_git(),
    ) else {
      panic!("expected the same error back");
    };
    assert_eq!(message, "x");
  }

  #[test]
  fn hard_denials_leave_out_the_optional_ones() {
    let denials = [denial(PermissionKind::Exec, "git"), denial(PermissionKind::Exec, "curl")];
    let (hard, soft) = split_denials(&denials, &optional_git());
    assert_eq!(hard, [denial(PermissionKind::Exec, "curl")]);
    assert_eq!(soft, [denial(PermissionKind::Exec, "git")]);
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
