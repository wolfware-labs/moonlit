use serde::Serialize;

/// The kind of capability a plugin asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PermissionKind {
  Network,
  Exec,
  Env,
  FilesystemRead,
  FilesystemWrite,
}

/// A permission a plugin declares in its metadata.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeclaredPermission {
  pub kind: PermissionKind,
  pub values: Vec<String>,
  pub reason: String,
  pub optional: bool,
}
