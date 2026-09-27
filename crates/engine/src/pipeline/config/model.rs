use crate::pipeline::config::span::Spanned;
use indexmap::IndexMap;

pub type ConfigMap = IndexMap<String, Spanned<ConfigValue>>;

#[derive(Clone, Debug, PartialEq)]
pub enum ConfigValue {
  Null,
  String(String),
  List(Vec<Spanned<ConfigValue>>),
  Map(ConfigMap),
}

#[derive(Clone, Debug, PartialEq)]
pub struct PipelineConfig {
  pub name: String,
  pub arguments: IndexMap<String, String>,
  pub variables: IndexMap<String, String>,
  pub plugins: Spanned<Vec<Plugin>>,
  pub stages: Spanned<Vec<Stage>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stage {
  pub name: String,
  pub steps: Vec<Step>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Plugin {
  pub name: String,
  pub url: Spanned<PluginUrl>,
  pub config: ConfigMap,
  pub permissions: Option<Permissions>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PluginUrl {
  Oci(String),
  File(String),
  Http(String),
  Https(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Step {
  pub name: String,
  pub run: Spanned<Run>,
  pub condition: Option<String>,
  pub halt_if: Option<String>,
  pub continue_on_error: bool,
  pub config: ConfigMap,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Run {
  pub plugin: String,
  pub middleware: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Permissions {
  pub network: Vec<String>,
  pub exec: Vec<String>,
  pub env: Vec<String>,
  pub filesystem: FilesystemAccess,
}

impl Permissions {
  pub fn full_trust() -> Self {
    Self {
      network: vec!["*".to_string()],
      exec: vec!["*".to_string()],
      env: vec!["*".to_string()],
      filesystem: FilesystemAccess::ReadWrite,
    }
  }

  pub fn deny() -> Self {
    Self {
      network: vec![],
      exec: vec![],
      env: vec![],
      filesystem: FilesystemAccess::None,
    }
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilesystemAccess {
  None,
  ReadOnly,
  ReadWrite,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn permissions_full_trust_defaults() {
    let p = Permissions::full_trust();
    assert_eq!(p.network, vec!["*".to_string()]);
    assert_eq!(p.exec, vec!["*".to_string()]);
    assert_eq!(p.env, vec!["*".to_string()]);
    assert_eq!(p.filesystem, FilesystemAccess::ReadWrite);
  }

  #[test]
  fn permissions_deny_grants_nothing() {
    let p = Permissions::deny();
    assert!(p.network.is_empty());
    assert!(p.exec.is_empty());
    assert!(p.env.is_empty());
    assert_eq!(p.filesystem, FilesystemAccess::None);
  }
}
