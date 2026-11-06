use std::time::Duration;

use moonlit_engine::pipeline::IncompatiblePlugin;
use moonlit_engine::plugin::publish::PLUGIN_WORLD;
use moonlit_engine::registry::{REGISTRY_TIMEOUT, RegistryClient, RegistryError, VerificationStatus, VersionRule};
use semver::Version;

use crate::display::display_safe;
use crate::plugin_spec::{RegistryRef, registry_ref};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Suggestion {
  Version {
    version: String,
    major: bool,
    status: VerificationStatus,
  },
  NoneCompatible,
  NotFromRegistry,
  RegistryUnavailable,
  RegistryDenied {
    host: String,
    status: u16,
  },
  RegistryError {
    text: String,
  },
}

/// Looks up, for each incompatible plugin, what the user can do about it; registry lookups happen only for `oci://` URLs.
pub async fn suggestions(list: &[IncompatiblePlugin], offline: bool) -> Vec<(IncompatiblePlugin, Suggestion)> {
  suggestions_within(list, offline, REGISTRY_TIMEOUT).await
}

async fn suggestions_within(
  list: &[IncompatiblePlugin],
  offline: bool,
  timeout: Duration,
) -> Vec<(IncompatiblePlugin, Suggestion)> {
  let mut answers: Vec<Option<Suggestion>> = vec![None; list.len()];
  let mut by_host: Vec<(String, Vec<(usize, RegistryRef)>)> = Vec::new();
  for (index, plugin) in list.iter().enumerate() {
    let Some(reference) = registry_ref(&plugin.url) else {
      answers[index] = Some(Suggestion::NotFromRegistry);
      continue;
    };
    if offline {
      answers[index] = Some(Suggestion::RegistryUnavailable);
      continue;
    }
    match by_host.iter_mut().find(|(host, _)| *host == reference.host) {
      Some((_, group)) => group.push((index, reference)),
      None => by_host.push((reference.host.clone(), vec![(index, reference)])),
    }
  }

  let mut hosts = tokio::task::JoinSet::new();
  for (host, group) in by_host {
    hosts.spawn(async move {
      let client = match RegistryClient::for_host(&host) {
        Ok(client) => client.with_timeout(timeout),
        Err(error) => {
          let text = error.to_string();
          return group
            .into_iter()
            .map(|(index, _)| (index, Suggestion::RegistryError { text: text.clone() }))
            .collect();
        }
      };
      let mut out = Vec::with_capacity(group.len());
      let mut unreachable = false;
      for (index, reference) in group {
        if unreachable {
          out.push((index, Suggestion::RegistryUnavailable));
          continue;
        }
        let suggestion = match lookup_suggestion(&client, &reference).await {
          Ok(suggestion) => suggestion,
          Err(error) => {
            unreachable = matches!(error, RegistryError::Unreachable(_));
            failed(&error, &reference.host)
          }
        };
        out.push((index, suggestion));
      }
      out
    });
  }
  while let Some(joined) = hosts.join_next().await {
    let group = joined.unwrap_or_else(|e| std::panic::resume_unwind(e.into_panic()));
    for (index, suggestion) in group {
      answers[index] = Some(suggestion);
    }
  }

  list
    .iter()
    .zip(answers)
    .map(|(plugin, suggestion)| (plugin.clone(), suggestion.unwrap_or(Suggestion::RegistryUnavailable)))
    .collect()
}

async fn lookup_suggestion(client: &RegistryClient, reference: &RegistryRef) -> Result<Suggestion, RegistryError> {
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
  if let Ok(current) = Version::parse(&reference.tag)
    && let Some(detail) = lookup(Some(current), false).await?
  {
    return Ok(Suggestion::Version {
      version: detail.version,
      major: false,
      status: detail.status,
    });
  }
  Ok(match lookup(None, true).await? {
    Some(detail) => Suggestion::Version {
      version: detail.version,
      major: true,
      status: detail.status,
    },
    None => Suggestion::NoneCompatible,
  })
}

fn failed(error: &RegistryError, host: &str) -> Suggestion {
  match error {
    RegistryError::NotARegistry(_) => Suggestion::NotFromRegistry,
    RegistryError::Denied { status, .. } => Suggestion::RegistryDenied {
      host: host.to_string(),
      status: *status,
    },
    RegistryError::Unreachable(_) => Suggestion::RegistryUnavailable,
    other => Suggestion::RegistryError { text: other.to_string() },
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
  if let Some(hint) = hint(suggestion, engine) {
    lines.push(format!("  {hint}"));
  }
  lines.iter().map(|line| display_safe(line)).collect::<Vec<_>>().join("\n") + "\n"
}

/// What the user can do about an incompatible plugin, for an engine supporting `engine`; `None` when the registry could not say.
#[must_use]
pub fn hint(suggestion: &Suggestion, engine: &str) -> Option<String> {
  match suggestion {
    Suggestion::Version { version, major, .. } => {
      let flag = if *major { " --major" } else { "" };
      Some(format!("use {version} instead (run `moonlit doctor{flag}` to fix)"))
    }
    Suggestion::NoneCompatible => Some(format!("no version of this plugin supports {engine} yet")),
    Suggestion::NotFromRegistry => Some(format!("rebuild it against moonlit-pdk for {engine}")),
    Suggestion::RegistryDenied { host, status } => Some(format!(
      "registry at {host} answered {status} on /graphql, no version to suggest"
    )),
    Suggestion::RegistryError { text } => Some(format!("the registry answered with an error: {text}")),
    Suggestion::RegistryUnavailable => None,
  }
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
          status: VerificationStatus::Verified,
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
    let denied = entry(Suggestion::RegistryDenied {
      host: "public.ecr.aws".to_string(),
      status: 401,
    });
    assert!(
      denied.contains("registry at public.ecr.aws answered 401 on /graphql, no version to suggest\n"),
      "{denied}"
    );
    let answered = entry(Suggestion::RegistryError {
      text: "registry returned an error: boom".to_string(),
    });
    assert!(
      answered.contains("  the registry answered with an error: registry returned an error: boom\n"),
      "{answered}"
    );
  }

  #[tokio::test]
  async fn plugins_on_a_silent_registry_wait_for_one_timeout_together() {
    use std::time::{Duration, Instant};
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
      .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(10)))
      .mount(&server)
      .await;
    let list: Vec<IncompatiblePlugin> = ["a", "b", "c"]
      .iter()
      .map(|name| IncompatiblePlugin {
        name: (*name).to_string(),
        url: format!("oci://{}/acme/{name}:1.0.0", server.address()),
        plugin_world: Some("moonlit:plugin@0.3.0".to_string()),
        engine_world: "moonlit:plugin@0.4.0".to_string(),
      })
      .collect();

    let started = Instant::now();
    let entries = suggestions_within(&list, false, Duration::from_millis(300)).await;

    assert!(started.elapsed() < Duration::from_secs(5), "{:?}", started.elapsed());
    assert!(entries.iter().all(|(_, s)| *s == Suggestion::RegistryUnavailable));
    assert_eq!(
      entries.iter().map(|(p, _)| p.name.as_str()).collect::<Vec<_>>(),
      ["a", "b", "c"]
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
  }

  #[tokio::test]
  async fn a_server_error_does_not_stop_the_other_plugins_on_the_host_from_being_asked() {
    use std::time::Duration;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
      .respond_with(ResponseTemplate::new(500))
      .mount(&server)
      .await;
    let list: Vec<IncompatiblePlugin> = ["a", "b", "c"]
      .iter()
      .map(|name| IncompatiblePlugin {
        name: (*name).to_string(),
        url: format!("oci://{}/acme/{name}:1.0.0", server.address()),
        plugin_world: Some("moonlit:plugin@0.3.0".to_string()),
        engine_world: "moonlit:plugin@0.4.0".to_string(),
      })
      .collect();

    let entries = suggestions_within(&list, false, Duration::from_secs(5)).await;

    assert_eq!(server.received_requests().await.unwrap().len(), 3);
    for (_, suggestion) in &entries {
      let line = hint(suggestion, "moonlit:plugin@0.4.0").unwrap_or_default();
      assert!(line.starts_with("the registry answered with an error: "), "{suggestion:?}");
      assert!(line.contains("500"), "{line}");
    }
  }
}
