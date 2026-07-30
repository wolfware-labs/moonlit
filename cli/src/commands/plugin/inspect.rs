//! `moonlit plugin inspect <path>` — validate a local component and print its
//! metadata + middlewares by instantiating it with zero capability grants.

use moonlit_engine::cache::Cache;
use moonlit_engine::host::{MiddlewareInfo, PluginMetadata};
use moonlit_engine::resolve::{PluginSource, ResolveOptions, resolve};

use super::introspect::introspect;
use crate::cli::{OutputMode, PluginInspectArgs};
use crate::render::resolve_mode;

pub async fn run(output: Option<OutputMode>, args: PluginInspectArgs) -> i32 {
    let bytes = if let Ok(source) = PluginSource::parse(&args.target) {
        // A recognized scheme (oci/file/http/https) → resolve (and pull if needed).
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
        // Not a scheme → a local filesystem path.
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

    let stdout_tty = std::io::IsTerminal::is_terminal(&std::io::stdout());
    match resolve_mode(output, stdout_tty) {
        OutputMode::Json => print_json(&meta, &mws),
        OutputMode::Plain => print_plain(&meta, &mws),
        OutputMode::Pretty => print_pretty(&meta, &mws),
    }
    0
}

fn print_pretty(meta: &PluginMetadata, mws: &[MiddlewareInfo]) {
    use comfy_table::{Table, presets::UTF8_BORDERS_ONLY};
    println!("{} v{}", meta.name, meta.version);
    if !meta.description.is_empty() {
        println!("{}", meta.description);
    }
    let mut table = Table::new();
    table.load_preset(UTF8_BORDERS_ONLY);
    table.set_header(["Middleware", "Description"]);
    for m in mws {
        table.add_row([m.name.as_str(), m.description.as_str()]);
    }
    println!("{table}");
}

fn print_plain(meta: &PluginMetadata, mws: &[MiddlewareInfo]) {
    println!("name: {}", meta.name);
    println!("version: {}", meta.version);
    println!("description: {}", meta.description);
    println!("middlewares:");
    for m in mws {
        println!("  {} - {}", m.name, m.description);
    }
}

fn print_json(meta: &PluginMetadata, mws: &[MiddlewareInfo]) {
    let v = serde_json::json!({
        "name": meta.name,
        "version": meta.version,
        "description": meta.description,
        // Data URI string, or null when the plugin declares no icon.
        "icon": meta.icon,
        "middlewares": mws.iter().map(|m| serde_json::json!({
            "name": m.name,
            "description": m.description,
            // The config schema travels the ABI as JSON text; re-embed it as an
            // object here (null when absent or unparseable) so the registry can
            // consume it directly.
            "configSchema": m.config_schema.as_deref()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok()),
        })).collect::<Vec<_>>(),
    });
    println!("{}", serde_json::to_string_pretty(&v).unwrap());
}
