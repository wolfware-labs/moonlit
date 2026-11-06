use crate::cli::{OutputMode, PluginInspectArgs};
use crate::display::display_safe;
use crate::plugin_probe::introspect;
use crate::render::resolve_mode;
use moonlit_engine::cache::Cache;
use moonlit_engine::plugin::PluginMetadata;
use moonlit_engine::plugin::middleware::MiddlewareInfo;
use moonlit_engine::plugin::permissions::DeclaredPermission;
use moonlit_engine::plugin::publish::PLUGIN_WORLD;
use moonlit_engine::plugin::resolver::{PluginSource, ResolveOptions, resolve};
use moonlit_engine::plugin::world::component_world;

#[must_use]
pub async fn run(output: Option<OutputMode>, args: PluginInspectArgs) -> i32 {
  let bytes = if let Ok(source) = &args.target.parse::<PluginSource>() {
    let cache = match Cache::new() {
      Ok(c) => c,
      Err(e) => {
        eprintln!("error: {e}");
        return 3;
      }
    };
    let resolved = match resolve(source, &ResolveOptions::default(), &cache, None).await {
      Ok(r) => r,
      Err(e) => {
        eprintln!("error: {e}");
        return 3;
      }
    };
    match std::fs::read(&resolved.wasm_path) {
      Ok(b) => b,
      Err(e) => {
        eprintln!("error: cannot read resolved plugin: {e}");
        return 3;
      }
    }
  } else {
    match std::fs::read(&args.target) {
      Ok(b) => b,
      Err(e) => {
        eprintln!("error: cannot read '{}': {e}", args.target);
        return 2;
      }
    }
  };

  match super::wasm::is_component(&bytes) {
    Ok(true) => {}
    Ok(false) => {
      eprintln!(
        "error: '{}' is a core wasm module, not a WASI-P2 component (build with `moonlit plugin build`)",
        args.target
      );
      return 2;
    }
    Err(e) => {
      eprintln!("error: {e}");
      return 2;
    }
  }
  if let Err(e) = super::wasm::validate(&bytes) {
    eprintln!("error: {e}");
    return 2;
  }

  let world = match component_world(&bytes) {
    Ok(w) => w,
    Err(e) => {
      eprintln!("error: {e}");
      return 2;
    }
  };
  if let Some(found) = world.as_deref().filter(|w| *w != PLUGIN_WORLD) {
    eprintln!("error: '{}' targets {found}; this CLI supports {PLUGIN_WORLD}", args.target);
    return 2;
  }

  let (meta, mws) = match introspect(&bytes).await {
    Ok(pair) => pair,
    Err(e) => {
      eprintln!("error: failed to instantiate component: {e}");
      return 3;
    }
  };

  match resolve_mode(output) {
    OutputMode::Json => print_json(&meta, &mws, world.as_deref()),
    OutputMode::Plain => print_plain(&meta, &mws),
    OutputMode::Pretty => print_pretty(&meta, &mws),
  }
  0
}

fn print_pretty(plugin_metadata: &PluginMetadata, plugin_middlewares: &[MiddlewareInfo]) {
  use comfy_table::{Table, presets::UTF8_BORDERS_ONLY};
  println!(
    "{} v{}",
    display_safe(&plugin_metadata.name),
    display_safe(&plugin_metadata.version)
  );
  if !plugin_metadata.description.is_empty() {
    println!("{}", display_safe(&plugin_metadata.description));
  }
  let mut table = Table::new();
  table.load_style(UTF8_BORDERS_ONLY);
  table.set_header(["Middleware", "Description"]);
  for m in plugin_middlewares {
    table.add_row([display_safe(&m.name), display_safe(&m.description)]);
  }
  println!("{table}");
  if !plugin_metadata.permissions.is_empty() {
    println!("Permissions:");
    for p in &plugin_metadata.permissions {
      println!("  {}", permission_line(p));
    }
  }
}

fn permission_line(p: &DeclaredPermission) -> String {
  let mut line = p.kind.sentence().to_string();
  if !p.values.is_empty() {
    line.push_str(" (");
    line.push_str(&p.values.join(", "));
    line.push(')');
  }
  if p.optional {
    line.push_str(" [optional]");
  }
  if !p.reason.is_empty() {
    line.push_str(": ");
    line.push_str(&p.reason);
  }
  display_safe(&line)
}

fn print_plain(plugin_metadata: &PluginMetadata, plgin_middlewares: &[MiddlewareInfo]) {
  println!("name: {}", display_safe(&plugin_metadata.name));
  println!("version: {}", display_safe(&plugin_metadata.version));
  println!("description: {}", display_safe(&plugin_metadata.description));
  println!("middlewares:");
  for m in plgin_middlewares {
    println!("  {} - {}", display_safe(&m.name), display_safe(&m.description));
  }
  println!("permissions:");
  for p in &plugin_metadata.permissions {
    println!("  {}", permission_line(p));
  }
}

fn print_json(plugin_metadata: &PluginMetadata, plugin_middlewares: &[MiddlewareInfo], world: Option<&str>) {
  let v = serde_json::json!({
      "name": plugin_metadata.name,
      "version": plugin_metadata.version,
      "description": plugin_metadata.description,
      "icon": plugin_metadata.icon,
      "world": world,
      "permissions": plugin_metadata.permissions,
      "middlewares": plugin_middlewares.iter().map(|m| serde_json::json!({
          "name": m.name,
          "description": m.description,
          "inputSchema": m.input_schema.as_deref()
              .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok()),
          "outputSchema": m.output_schema.as_deref()
              .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok()),
      })).collect::<Vec<_>>(),
  });
  println!("{}", serde_json::to_string_pretty(&v).unwrap());
}
