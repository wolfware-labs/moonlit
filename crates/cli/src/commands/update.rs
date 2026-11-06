use std::path::Path;

use moonlit_engine::pipeline::config::{Permissions, parse_config};
use moonlit_engine::plugin::permissions::{DeclaredPermission, grants_for, is_covered, merge_grants, uncovered_required};
use moonlit_engine::plugin::publish::PLUGIN_WORLD;
use moonlit_engine::registry::{RegistryClient, RegistryError, VerificationStatus, VersionRule};
use semver::Version;

use crate::cli::{OutputMode, UpdateArgs};
use crate::consent::{self, Answer, ConsentView};
use crate::display::display_safe;
use crate::manifest::resolve_manifest_path;
use crate::plugin_probe::probe;
use crate::plugin_spec::{RegistryRef, registry_ref};
use crate::release_file::{render_diff, set_plugin, set_plugin_url};
use crate::render::resolve_mode;

pub(crate) const REFUSAL: &str =
  "refusing to grant permissions without a terminal; pass --yes to accept the plugin's required permissions";

enum Planned {
  Bump(Box<Plan>),
  UpToDate,
  NotARegistry,
}

struct Plan {
  name: String,
  old: String,
  new: String,
  url: String,
  permissions: Permissions,
  uncovered: Vec<DeclaredPermission>,
  status: VerificationStatus,
  description: String,
}

#[must_use]
pub async fn run(output: Option<OutputMode>, args: UpdateArgs) -> i32 {
  match update(output, &args).await {
    Ok(code) | Err(code) => code,
  }
}

async fn update(output: Option<OutputMode>, args: &UpdateArgs) -> Result<i32, i32> {
  let path = resolve_manifest_path(args.file_path.as_deref()).or_else(|e| fail(&e.to_string(), 2))?;
  let before = std::fs::read_to_string(&path).or_else(|e| fail(&format!("cannot read '{}': {e}", path.display()), 2))?;
  let config = parse_config(&before, "release.yml").or_else(|e| fail(e.message(), 2))?;
  let plugins = &config.plugins.value;
  if let Some(wanted) = &args.plugin
    && !plugins.iter().any(|p| &p.name == wanted)
  {
    return fail(&format!("no plugin named '{wanted}' is declared"), 2);
  }

  let mut plans = Vec::new();
  for plugin in plugins.iter().filter(|p| args.plugin.as_ref().is_none_or(|w| *w == p.name)) {
    let Some(reference) = registry_ref(plugin.url.value.as_str()) else {
      if args.plugin.is_some() {
        return fail(&format!("'{}' does not come from a registry", plugin.name), 2);
      }
      continue;
    };
    let Ok(current) = Version::parse(&reference.tag) else {
      println!("{}: pinned to {}, skipped", plugin.name, reference.tag);
      continue;
    };
    let granted = plugin.permissions.clone().unwrap_or_else(Permissions::deny);
    match plan(&plugin.name, &reference, &current, &granted, args).await? {
      Planned::Bump(plan) => plans.push(*plan),
      Planned::UpToDate => println!("{}: up to date", plugin.name),
      Planned::NotARegistry => println!("{}: not from a Moonlit registry, skipped", plugin.name),
    }
  }

  if !args.yes && !consent::interactive() && plans.iter().any(|p| !p.uncovered.is_empty()) {
    return fail(REFUSAL, 2);
  }

  let color = resolve_mode(output) == OutputMode::Pretty;
  let file = display_path(args.file_path.as_deref(), &path);
  let mut content = before.clone();
  let mut declined = false;
  for plan in &plans {
    let edited = if plan.uncovered.is_empty() {
      set_plugin_url(&content, &plan.name, &plan.url)
    } else {
      set_plugin(&content, &plan.name, &plan.url, &plan.permissions)
    };
    let after = edited.or_else(|e| fail(&e.to_string(), 2))?;
    if !plan.uncovered.is_empty() {
      let view = ConsentView {
        plugin: &plan.name,
        version: &plan.new,
        status: Some(plan.status),
        description: &plan.description,
        required: plan.uncovered.iter().collect(),
        optional: Vec::new(),
        not_requested: false,
      };
      print!("{}", consent::render(&view, color));
      print!("{}", consent::render_changes(&file, &render_diff(&content, &after)));
      if !args.yes && consent::ask("Allow and update?").or_else(|e| prompt_failed(&e))? == Answer::No {
        println!("{}: kept at {}", plan.name, plan.old);
        declined = true;
        continue;
      }
    }
    content = after;
    println!("{}: {} → {}", plan.name, plan.old, display_safe(&plan.new));
  }

  if content != before {
    std::fs::write(&path, &content).or_else(|e| fail(&format!("cannot write '{}': {e}", path.display()), 2))?;
  }
  Ok(i32::from(declined))
}

async fn plan(
  name: &str,
  reference: &RegistryRef,
  current: &Version,
  granted: &Permissions,
  args: &UpdateArgs,
) -> Result<Planned, i32> {
  let rule = VersionRule {
    current: Some(current.clone()),
    cross_major: args.major,
    pre: args.pre,
  };
  let detail = match RegistryClient::for_host(&reference.host)
    .newest_compatible(&reference.org, &reference.name, PLUGIN_WORLD, rule)
    .await
  {
    Ok(detail) => detail,
    Err(RegistryError::NotARegistry(_)) => return Ok(Planned::NotARegistry),
    Err(e) => return fail(&format!("{name}: {e}"), 3),
  };
  let Some(detail) = detail else {
    return Ok(Planned::UpToDate);
  };
  let newer = Version::parse(&detail.version).is_ok_and(|v| v > *current);
  if !newer {
    return Ok(Planned::UpToDate);
  }
  let url = format!(
    "oci://{}/{}/{}:{}",
    reference.host, reference.org, reference.name, detail.version
  );
  let probed = probe(&url).await.or_else(|e| fail(&format!("{name}: {e}"), 3))?;
  let declared = &probed.meta.permissions;
  let uncovered: Vec<DeclaredPermission> = uncovered_required(declared, granted).into_iter().cloned().collect();
  for p in declared.iter().filter(|p| p.optional && !is_covered(p, granted)) {
    println!(
      "{name}: new optional permission available: {} ({})",
      p.kind.sentence(),
      consent::values_text(p)
    );
  }
  let permissions = if uncovered.is_empty() {
    granted.clone()
  } else {
    let wanted = grants_for(&uncovered, &|_| false);
    merge_grants(granted, &wanted)
  };
  Ok(Planned::Bump(Box::new(Plan {
    name: name.to_string(),
    old: current.to_string(),
    new: detail.version,
    status: detail.status,
    url,
    permissions,
    uncovered,
    description: probed.meta.description.clone(),
  })))
}

pub(crate) fn fail<T>(message: &str, code: i32) -> Result<T, i32> {
  eprintln!("error: {}", display_safe(message));
  Err(code)
}

pub(crate) fn display_path(given: Option<&Path>, resolved: &Path) -> String {
  match given {
    Some(given) => given.display().to_string(),
    None => resolved
      .file_name()
      .map_or_else(|| resolved.display().to_string(), |n| n.to_string_lossy().into_owned()),
  }
}

pub(crate) fn prompt_failed<T>(e: &std::io::Error) -> Result<T, i32> {
  if e.kind() == std::io::ErrorKind::Interrupted {
    println!("Nothing changed.");
    Err(1)
  } else {
    fail(&e.to_string(), 2)
  }
}
