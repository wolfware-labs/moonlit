//! `moonlit plugin inspect <path>` — validate a local component and print its
//! metadata + middlewares by instantiating it with zero capability grants.

use moonlit_engine::host::{MiddlewareInfo, PluginMetadata};

use super::introspect::introspect;
use crate::cli::{OutputMode, PluginInspectArgs};
use crate::render::resolve_mode;

pub async fn run(output: Option<OutputMode>, args: PluginInspectArgs) -> i32 {
    let bytes = match std::fs::read(&args.path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: cannot read '{}': {e}", args.path.display());
            return 2;
        }
    };

    match super::wasm::is_component(&bytes) {
        Ok(true) => {}
        Ok(false) => {
            eprintln!(
                "error: '{}' is a core wasm module, not a WASI-P2 component (build with `moonlit plugin build`)",
                args.path.display()
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
        "middlewares": mws.iter().map(|m| serde_json::json!({
            "name": m.name,
            "description": m.description,
        })).collect::<Vec<_>>(),
    });
    println!("{}", serde_json::to_string_pretty(&v).unwrap());
}
