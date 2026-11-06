use moonlit_engine::pipeline::IncompatiblePlugin;
use moonlit_engine::pipeline::config::{Permissions, parse_config};
use moonlit_engine::plugin::permissions::{DeclaredPermission, grant_missing, uncovered_required};
use moonlit_engine::plugin::publish::PLUGIN_WORLD;
use moonlit_engine::registry::VerificationStatus;

use crate::cli::{DoctorArgs, OutputMode};
use crate::commands::common::{PERMISSION_REFUSAL, display_path, fail, prompt_failed, read_release, write_release};
use crate::compat::{self, Suggestion};
use crate::consent::{self, Answer, ConsentView};
use crate::display::display_safe;
use crate::plugin_probe::{ProbeError, probe};
use crate::plugin_spec::{RegistryRef, registry_ref};
use crate::release_file::set_plugin;
use crate::render::resolve_mode;

struct Fix {
  name: String,
  url: String,
  moved: Option<(String, String)>,
  version: String,
  description: String,
  status: Option<VerificationStatus>,
  uncovered: Vec<DeclaredPermission>,
  permissions: Option<Permissions>,
}

enum Outcome {
  Healthy,
  Problem { text: String, fix: Option<Box<Fix>> },
  RegistryDown { text: String },
}

enum Unfixed {
  Problem(String),
  RegistryDown(String),
}

#[must_use]
pub async fn run(output: Option<OutputMode>, args: DoctorArgs) -> i32 {
  match doctor(output, &args).await {
    Ok(code) | Err(code) => code,
  }
}

async fn doctor(output: Option<OutputMode>, args: &DoctorArgs) -> Result<i32, i32> {
  let (path, before) = read_release(args.file_path.as_deref())?;
  let config = parse_config(&before, "release.yml").or_else(|e| fail(e.message(), 2))?;
  let file = display_path(args.file_path.as_deref(), &path);

  let mut fixes = Vec::new();
  let mut unfixed = false;
  let mut registry_down = false;
  for plugin in &config.plugins.value {
    let granted = plugin.permissions.clone().unwrap_or_else(Permissions::deny);
    match check(&plugin.name, plugin.url.value.as_str(), &granted, args.major).await {
      Outcome::Healthy => println!("✓ {}", plugin.name),
      Outcome::Problem { text, fix } => {
        println!("✗ {}: {}", plugin.name, display_safe(&text));
        match fix {
          Some(fix) => fixes.push(*fix),
          None => unfixed = true,
        }
      }
      Outcome::RegistryDown { text } => {
        println!("✗ {}: {}", plugin.name, display_safe(&text));
        registry_down = true;
      }
    }
  }

  if registry_down {
    return fail("cannot reach the registry; nothing was changed", 3);
  }
  if fixes.is_empty() {
    if !unfixed {
      println!("{file} looks good.");
    }
    return Ok(i32::from(unfixed));
  }

  let grants = fixes.iter().any(|f| f.permissions.is_some());
  if grants && !args.yes && !consent::interactive() {
    return fail(PERMISSION_REFUSAL, 2);
  }

  let mut content = before.clone();
  for fix in &fixes {
    content = set_plugin(&content, &fix.name, &fix.url, fix.permissions.as_ref()).or_else(|e| fail(&e.to_string(), 2))?;
  }

  let color = resolve_mode(output) == OutputMode::Pretty;
  println!();
  for fix in fixes.iter().filter(|f| f.permissions.is_some()) {
    let view = ConsentView {
      plugin: &fix.name,
      version: &fix.version,
      status: fix.status,
      description: &fix.description,
      required: fix.uncovered.iter().collect(),
      optional: Vec::new(),
      not_requested: false,
    };
    print!("{}", consent::render(&view, color));
  }
  print!("{}", consent::render_changes(&file, &before, &content));
  if !args.yes && consent::interactive() && consent::ask("Allow and update?").or_else(|e| prompt_failed(&e))? == Answer::No {
    println!("Nothing changed.");
    return Ok(1);
  }

  write_release(&path, &content)?;
  for fix in &fixes {
    match &fix.moved {
      Some((old, new)) => println!("{}: {old} → {}", fix.name, display_safe(new)),
      None => println!("{}: permissions granted", fix.name),
    }
  }
  Ok(i32::from(unfixed))
}

async fn check(name: &str, url: &str, granted: &Permissions, major: bool) -> Outcome {
  match probe(url).await {
    Ok(probed) => {
      let uncovered = uncovered(&probed.meta.permissions, granted);
      if uncovered.is_empty() {
        return Outcome::Healthy;
      }
      Outcome::Problem {
        text: format!("not granted: {}", describe(&uncovered)),
        fix: Some(Box::new(Fix {
          name: name.to_string(),
          url: url.to_string(),
          moved: None,
          version: probed.meta.version.clone(),
          description: probed.meta.description.clone(),
          status: None,
          permissions: Some(grant_missing(granted, &uncovered)),
          uncovered,
        })),
      }
    }
    Err(ProbeError::Incompatible { mismatch, .. }) => {
      let text = mismatch.to_string();
      match moved(name, url, mismatch.found, granted, major).await {
        Ok(fix) => Outcome::Problem {
          text,
          fix: Some(Box::new(fix)),
        },
        Err(Unfixed::Problem(why)) => Outcome::Problem {
          text: format!("{text}; {why}"),
          fix: None,
        },
        Err(Unfixed::RegistryDown(why)) => Outcome::RegistryDown {
          text: format!("{text}; {why}"),
        },
      }
    }
    Err(e @ ProbeError::Resolve { network: true, .. }) => Outcome::RegistryDown { text: e.to_string() },
    Err(e) => Outcome::Problem {
      text: e.to_string(),
      fix: None,
    },
  }
}

async fn moved(name: &str, url: &str, found: Option<String>, granted: &Permissions, major: bool) -> Result<Fix, Unfixed> {
  if let Some(pinned) = registry_ref(url).filter(RegistryRef::is_digest) {
    return Err(Unfixed::Problem(format!(
      "pinned to {}; change the url to a version tag to move it",
      pinned.tag
    )));
  }
  let incompatible = IncompatiblePlugin {
    name: name.to_string(),
    url: url.to_string(),
    plugin_world: found,
    engine_world: PLUGIN_WORLD.to_string(),
  };
  let suggestion = compat::suggestions(&[incompatible], false)
    .await
    .pop()
    .map_or(Suggestion::RegistryUnavailable, |(_, s)| s);
  let (version, status) = match &suggestion {
    Suggestion::Version {
      version,
      major: crosses,
      status,
    } if major || !crosses => (version.clone(), *status),
    Suggestion::RegistryUnavailable => return Err(Unfixed::RegistryDown("cannot reach the registry".to_string())),
    other => return Err(Unfixed::Problem(compat::hint(other, PLUGIN_WORLD).unwrap_or_default())),
  };
  let reference = registry_ref(url).ok_or_else(|| Unfixed::Problem("does not come from a registry".to_string()))?;
  let target = reference.with_tag(&version);
  let probed = probe(&target).await.map_err(|e| match e {
    ProbeError::Resolve {
      network: true, message, ..
    } => Unfixed::RegistryDown(format!("cannot fetch {target}: {message}")),
    ProbeError::Resolve { message, .. } => Unfixed::Problem(format!("cannot fetch {target}: {message}")),
    other => Unfixed::Problem(other.to_string()),
  })?;
  let uncovered = uncovered(&probed.meta.permissions, granted);
  let permissions = (!uncovered.is_empty()).then(|| grant_missing(granted, &uncovered));
  Ok(Fix {
    name: name.to_string(),
    url: target,
    moved: Some((reference.tag, version.clone())),
    version,
    description: probed.meta.description.clone(),
    status: Some(status),
    uncovered,
    permissions,
  })
}

fn uncovered(declared: &[DeclaredPermission], granted: &Permissions) -> Vec<DeclaredPermission> {
  uncovered_required(declared, granted).into_iter().cloned().collect()
}

fn describe(permissions: &[DeclaredPermission]) -> String {
  permissions
    .iter()
    .map(|p| format!("{} ({})", p.kind.sentence(), consent::values_text(p)))
    .collect::<Vec<_>>()
    .join(", ")
}
