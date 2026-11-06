use moonlit_engine::plugin::permissions::{DeclaredPermission, grants_for};
use moonlit_engine::plugin::publish::PLUGIN_WORLD;
use moonlit_engine::registry::{RegistryClient, VerificationStatus, VersionDetail, VersionRule};

use crate::cli::{AddArgs, DEFAULT_REGISTRY_HOST, OutputMode};
use crate::commands::common::{PERMISSION_REFUSAL, display_path, fail, prompt_failed};
use crate::consent::{self, Answer, ConsentView};
use crate::display::display_safe;
use crate::manifest::resolve_manifest_path;
use crate::plugin_probe::probe;
use crate::plugin_spec::{PluginSpec, RegistryRef, registry_ref};
use crate::release_file::{PluginEntry, ReleaseFileError, add_plugin, check_addable, render_diff};
use crate::render::resolve_mode;

struct Target {
  url: String,
  label: Option<String>,
  version: Option<String>,
  status: Option<VerificationStatus>,
}

#[must_use]
pub async fn run(output: Option<OutputMode>, args: AddArgs) -> i32 {
  match add(output, &args).await {
    Ok(code) | Err(code) => code,
  }
}

async fn add(output: Option<OutputMode>, args: &AddArgs) -> Result<i32, i32> {
  if !args.yes && !consent::interactive() {
    return fail(PERMISSION_REFUSAL, 2);
  }
  let spec = PluginSpec::parse(&args.plugin).or_else(|e| fail(&e, 2))?;
  let path = resolve_manifest_path(args.file_path.as_deref()).or_else(|e| fail(&e.to_string(), 2))?;
  let before = std::fs::read_to_string(&path).or_else(|e| fail(&format!("cannot read '{}': {e}", path.display()), 2))?;
  let target = resolve_target(spec, args.registry.as_deref()).await?;
  let probe = probe(&target.url).await.or_else(|e| fail(&e.to_string(), 3))?;

  let meta = &probe.meta;
  let name = args.name.clone().unwrap_or_else(|| {
    meta
      .name
      .rsplit_once('/')
      .map_or(meta.name.as_str(), |(_, last)| last)
      .to_string()
  });
  check_addable(&before, &name, &target.url).or_else(|e| fail(&clash_message(&e, &target.url), 2))?;
  let (optional, required): (Vec<&DeclaredPermission>, Vec<&DeclaredPermission>) =
    meta.permissions.iter().partition(|p| p.optional);
  let view = ConsentView {
    plugin: target.label.as_deref().unwrap_or(&meta.name),
    version: target.version.as_deref().unwrap_or(&meta.version),
    status: target.status,
    description: &meta.description,
    required,
    optional,
    not_requested: true,
  };
  print!("{}", consent::render(&view, resolve_mode(output) == OutputMode::Pretty));

  let accepted = accepted_optional(&view.optional, args)?;
  let entry = PluginEntry {
    name: name.clone(),
    url: target.url.clone(),
    permissions: grants_for(&meta.permissions, &|p| accepted.contains(&p)),
  };
  let after = add_plugin(&before, &entry).or_else(|e| fail(&clash_message(&e, &target.url), 2))?;
  let file = display_path(args.file_path.as_deref(), &path);
  print!("{}", consent::render_changes(&file, &render_diff(&before, &after)));

  if !args.yes && consent::ask("Allow and add?").or_else(|e| prompt_failed(&e))? == Answer::No {
    println!("Nothing changed.");
    return Ok(1);
  }
  std::fs::write(&path, &after).or_else(|e| fail(&format!("cannot write '{}': {e}", path.display()), 2))?;
  println!("Added {} to {file}.", display_safe(&name));
  Ok(0)
}

async fn resolve_target(spec: PluginSpec, registry: Option<&str>) -> Result<Target, i32> {
  match spec {
    PluginSpec::Url(url) => Ok(Target {
      url,
      label: None,
      version: None,
      status: None,
    }),
    PluginSpec::Registry { org, name, version } => {
      let registry = registry.unwrap_or(DEFAULT_REGISTRY_HOST);
      let detail = registry_version(registry, &org, &name, version.as_deref())
        .await
        .or_else(|message| fail(&message, 3))?;
      let reference = RegistryRef {
        host: oci_host(registry).to_string(),
        org: org.clone(),
        name: name.clone(),
        tag: detail.version.clone(),
      };
      Ok(Target {
        url: reference.with_tag(&detail.version),
        label: Some(format!("{org}/{name}")),
        version: Some(detail.version),
        status: Some(detail.status),
      })
    }
  }
}

fn clash_message(e: &ReleaseFileError, url: &str) -> String {
  match e {
    ReleaseFileError::NameTaken { name, url: declared } if same_plugin(declared, url) => {
      format!("{e}; run `moonlit update {name}` to change its version")
    }
    ReleaseFileError::NameTaken { .. } => format!("{e}; pass --name to add it under another name"),
    ReleaseFileError::UrlPresent { name } => format!("{e}; run `moonlit update {name}` to change its version"),
    e => e.to_string(),
  }
}

fn same_plugin(declared: &str, url: &str) -> bool {
  match (registry_ref(declared), registry_ref(url)) {
    (Some(a), Some(b)) => a.host == b.host && a.org == b.org && a.name == b.name,
    _ => declared == url,
  }
}

fn accepted_optional<'a>(optional: &[&'a DeclaredPermission], args: &AddArgs) -> Result<Vec<&'a DeclaredPermission>, i32> {
  if args.with_optional {
    return Ok(optional.to_vec());
  }
  if args.yes {
    return Ok(Vec::new());
  }
  let mut accepted = Vec::new();
  for p in optional {
    if consent::ask_optional(p).or_else(|e| prompt_failed(&e))? == Answer::Yes {
      accepted.push(*p);
    }
  }
  Ok(accepted)
}

async fn registry_version(registry: &str, org: &str, name: &str, version: Option<&str>) -> Result<VersionDetail, String> {
  let client = RegistryClient::for_host(registry);
  let Some(version) = version else {
    let rule = VersionRule {
      current: None,
      cross_major: true,
      pre: false,
    };
    return client
      .newest_compatible(org, name, PLUGIN_WORLD, rule)
      .await
      .map_err(|e| e.to_string())?
      .ok_or_else(|| format!("no version of {org}/{name} in the registry supports {PLUGIN_WORLD}"));
  };
  let detail = client
    .package_version(org, name, version)
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| format!("{org}/{name}@{version} is not in the registry"))?;
  match detail.world_version.as_deref() {
    Some(PLUGIN_WORLD) => Ok(detail),
    found => Err(format!(
      "{org}/{name}@{version} targets {}; this CLI supports {PLUGIN_WORLD}",
      found.unwrap_or("an unknown world")
    )),
  }
}

fn oci_host(registry: &str) -> &str {
  let rest = registry
    .strip_prefix("https://")
    .or_else(|| registry.strip_prefix("http://"))
    .unwrap_or(registry);
  rest.split('/').next().unwrap_or(rest)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn oci_host_drops_scheme_and_path() {
    assert_eq!(oci_host("registry.moonlit.rs"), "registry.moonlit.rs");
    assert_eq!(oci_host("https://registry.moonlit.rs/graphql"), "registry.moonlit.rs");
    assert_eq!(oci_host("http://127.0.0.1:5185"), "127.0.0.1:5185");
  }
}
