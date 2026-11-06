use moonlit_engine::pipeline::IncompatiblePlugin;
use moonlit_engine::plugin::publish::PLUGIN_WORLD;
use moonlit_engine::registry::{RegistryClient, VersionRule};
use semver::Version;

use crate::plugin_spec::registry_ref;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Suggestion {
  Version { version: String, major: bool },
  NoneCompatible,
  NotFromRegistry,
  RegistryUnavailable,
}

/// Looks up, for each incompatible plugin, what the user can do about it; registry lookups happen only for `oci://` URLs.
pub async fn suggestions(list: &[IncompatiblePlugin], offline: bool) -> Vec<(IncompatiblePlugin, Suggestion)> {
  let mut out = Vec::with_capacity(list.len());
  for plugin in list {
    out.push((plugin.clone(), suggest(plugin, offline).await));
  }
  out
}

async fn suggest(plugin: &IncompatiblePlugin, offline: bool) -> Suggestion {
  let Some(reference) = registry_ref(&plugin.url) else {
    return Suggestion::NotFromRegistry;
  };
  if offline {
    return Suggestion::RegistryUnavailable;
  }
  let client = RegistryClient::for_host(&reference.host);
  let lookup = |current: Option<Version>, cross_major: bool| {
    let rule = VersionRule {
      current,
      cross_major,
      pre: false,
    };
    let client = client.clone();
    let (org, name) = (reference.org.clone(), reference.name.clone());
    async move { client.newest_compatible(&org, &name, PLUGIN_WORLD, rule).await }
  };
  if let Ok(current) = Version::parse(&reference.tag) {
    match lookup(Some(current), false).await {
      Ok(Some(detail)) => {
        return Suggestion::Version {
          version: detail.version,
          major: false,
        };
      }
      Ok(None) => {}
      Err(_) => return Suggestion::RegistryUnavailable,
    }
  }
  match lookup(None, true).await {
    Ok(Some(detail)) => Suggestion::Version {
      version: detail.version,
      major: true,
    },
    Ok(None) => Suggestion::NoneCompatible,
    Err(_) => Suggestion::RegistryUnavailable,
  }
}

/// Renders one block per plugin: what it was built for, and what to do about it.
#[must_use]
pub fn render(entries: &[(IncompatiblePlugin, Suggestion)]) -> String {
  entries.iter().map(|(plugin, suggestion)| block(plugin, suggestion)).collect()
}

fn block(plugin: &IncompatiblePlugin, suggestion: &Suggestion) -> String {
  let built_for = plugin.plugin_world.as_deref().unwrap_or("an unknown plugin contract");
  let upgrade_cli = if world_is_newer(plugin.plugin_world.as_deref(), &plugin.engine_world) {
    " — upgrade the CLI"
  } else {
    ""
  };
  let engine = &plugin.engine_world;
  let mut lines = vec![
    format!("✗ {}  {}", plugin.name, plugin.url),
    format!("  built for {built_for}, this CLI supports {engine}{upgrade_cli}"),
  ];
  match suggestion {
    Suggestion::Version { version, major } => {
      let flag = if *major { " --major" } else { "" };
      lines.push(format!("  use {version} instead (run `moonlit doctor{flag}` to fix)"));
    }
    Suggestion::NoneCompatible => lines.push(format!("  no version of this plugin supports {engine} yet")),
    Suggestion::NotFromRegistry => lines.push(format!("  rebuild it against moonlit-pdk for {engine}")),
    Suggestion::RegistryUnavailable => {}
  }
  lines.join("\n") + "\n"
}

fn world_version(world: &str) -> Option<Version> {
  Version::parse(world.rsplit_once('@')?.1).ok()
}

fn world_is_newer(plugin: Option<&str>, engine: &str) -> bool {
  match (plugin.and_then(world_version), world_version(engine)) {
    (Some(p), Some(e)) => p > e,
    _ => false,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn plugin(world: Option<&str>) -> IncompatiblePlugin {
    IncompatiblePlugin {
      name: "p".to_string(),
      url: "file:///p.wasm".to_string(),
      plugin_world: world.map(str::to_string),
      engine_world: "moonlit:plugin@0.4.0".to_string(),
    }
  }

  #[test]
  fn newer_plugin_world_asks_for_a_cli_upgrade() {
    let text = render(&[(plugin(Some("moonlit:plugin@0.5.0")), Suggestion::RegistryUnavailable)]);
    assert!(
      text.contains("this CLI supports moonlit:plugin@0.4.0 — upgrade the CLI\n"),
      "{text}"
    );
    let older = render(&[(plugin(Some("moonlit:plugin@0.3.0")), Suggestion::RegistryUnavailable)]);
    assert!(!older.contains("upgrade the CLI"), "{older}");
  }

  #[test]
  fn version_suggestion_names_the_doctor_command_that_applies_it() {
    let line = |major| {
      render(&[(
        plugin(Some("moonlit:plugin@0.3.0")),
        Suggestion::Version {
          version: "2.0.0".to_string(),
          major,
        },
      )])
    };
    assert!(line(false).contains("use 2.0.0 instead (run `moonlit doctor` to fix)\n"));
    assert!(line(true).contains("use 2.0.0 instead (run `moonlit doctor --major` to fix)\n"));
  }

  #[test]
  fn each_suggestion_has_its_line() {
    let entry = |s| render(&[(plugin(Some("moonlit:plugin@0.3.0")), s)]);
    assert!(entry(Suggestion::NoneCompatible).contains("no version of this plugin supports moonlit:plugin@0.4.0 yet"));
    assert!(entry(Suggestion::NotFromRegistry).contains("rebuild it against moonlit-pdk for moonlit:plugin@0.4.0"));
    assert_eq!(entry(Suggestion::RegistryUnavailable).lines().count(), 2);
  }
}
