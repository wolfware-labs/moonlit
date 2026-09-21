use super::introspect::introspect;
use crate::cli::{OutputMode, PluginInspectArgs};
use crate::render::resolve_mode;
use moonlit_engine::cache::Cache;
use moonlit_engine::plugin::{MiddlewareInfo, PluginMetadata, PluginSource};

pub async fn run(output: Option<OutputMode>, args: PluginInspectArgs) -> i32 {
    let bytes = if let Ok(source) = &args.target.parse::<PluginSource>() {
        let cache = match Cache::new() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("error: {e}");
                return 3;
            }
        };
        let resolved = match resolve(&source, &ResolveOptions::default(), &cache, None).await {
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

    let (meta, mws) = match introspect(&bytes).await {
        Ok(pair) => pair,
        Err(e) => {
            eprintln!("error: failed to instantiate component: {e}");
            return 3;
        }
    };

    match resolve_mode(output) {
        OutputMode::Json => print_json(&meta, &mws),
        OutputMode::Plain => print_plain(&meta, &mws),
        OutputMode::Pretty => print_pretty(&meta, &mws),
    }
    0
}

fn print_pretty(plugin_metadata: &PluginMetadata, plugin_middlewares: &[MiddlewareInfo]) {
    use comfy_table::{Table, presets::UTF8_BORDERS_ONLY};
    println!("{} v{}", plugin_metadata.name, plugin_metadata.version);
    if !plugin_metadata.description.is_empty() {
        println!("{}", plugin_metadata.description);
    }
    let mut table = Table::new();
    table.load_style(UTF8_BORDERS_ONLY);
    table.set_header(["Middleware", "Description"]);
    for m in plugin_middlewares {
        table.add_row([m.name.as_str(), m.description.as_str()]);
    }
    println!("{table}");
}

fn print_plain(plugin_metadata: &PluginMetadata, plgin_middlewares: &[MiddlewareInfo]) {
    println!("name: {}", plugin_metadata.name);
    println!("version: {}", plugin_metadata.version);
    println!("description: {}", plugin_metadata.description);
    println!("middlewares:");
    for m in plgin_middlewares {
        println!("  {} - {}", m.name, m.description);
    }
}

fn print_json(plugin_metadata: &PluginMetadata, plugin_middlewares: &[MiddlewareInfo]) {
    let v = serde_json::json!({
        "name": plugin_metadata.name,
        "version": plugin_metadata.version,
        "description": plugin_metadata.description,
        "icon": plugin_metadata.icon,
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
