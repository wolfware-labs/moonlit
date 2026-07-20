//! `moonlit plugin inspect <path>` — validate a local component and print its
//! metadata + middlewares by instantiating it with zero capability grants.

use std::sync::Arc;

use moonlit_engine::config::model::{FilesystemAccess, Permissions};
use moonlit_engine::host::{
    HostEventSink, InstanceConfig, LogLevel, MiddlewareInfo, PluginInstance, PluginMetadata,
    test_engine,
};

use crate::cli::{OutputMode, PluginInspectArgs};
use crate::render::resolve_mode;

/// Inspect never needs guest logs; discard them.
struct SilentSink;
impl HostEventSink for SilentSink {
    fn log(&self, _step: &str, _level: LogLevel, _message: &str) {}
    fn progress(&self, _step: &str, _message: &str) {}
}

/// Zero grants: `init`/`list-middlewares` require no capabilities.
fn no_permissions() -> Permissions {
    Permissions {
        network: vec![],
        exec: vec![],
        env: vec![],
        filesystem: FilesystemAccess::None,
    }
}

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

async fn introspect(bytes: &[u8]) -> Result<(PluginMetadata, Vec<MiddlewareInfo>), String> {
    let engine = test_engine();
    let cfg = InstanceConfig {
        working_directory: std::env::temp_dir(),
        permissions: no_permissions(),
        config_view: serde_json::json!({}),
        env_snapshot: vec![],
    };
    let mut inst = PluginInstance::instantiate(&engine, bytes, cfg, Arc::new(SilentSink))
        .await
        .map_err(|e| e.to_string())?;
    let meta = inst.init(&serde_json::json!({})).await?;
    let mws = inst.list_middlewares().await.map_err(|e| e.to_string())?;
    Ok((meta, mws))
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
