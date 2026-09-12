//! `moonlit plugin new <name>` — scaffold a plugin crate from embedded templates.
//! Interactive (cliclack) on a TTY; flag/default-driven off a TTY.

use std::io::IsTerminal;
use std::path::Path;

use crate::cli::PluginNewArgs;

use super::scaffold::{ScaffoldValues, is_valid_crate_name, pdk_dep_line};
use super::templates::render_all;

pub fn run(args: PluginNewArgs) -> i32 {
    if !is_valid_crate_name(&args.name) {
        eprintln!(
            "error: '{}' is not a valid crate name (start with a letter; letters, digits, '-', '_')",
            args.name
        );
        return 2;
    }

    let target = Path::new(&args.name);
    if target.exists() {
        eprintln!("error: '{}' already exists", target.display());
        return 2;
    }

    let interactive = std::io::stdin().is_terminal() && std::io::stderr().is_terminal();
    let values = match resolve_values(&args, interactive) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };

    if let Err(e) = write_scaffold(target, &values) {
        eprintln!("error: {e}");
        return 1;
    }

    println!("✔ created plugin '{}' in {}/", values.name, values.name);
    println!("  next: cd {} && moonlit plugin build", values.name);
    0
}

fn default_namespace() -> String {
    std::process::Command::new("git")
        .args(["config", "user.name"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "my-org".to_string())
}

fn resolve_values(args: &PluginNewArgs, interactive: bool) -> std::io::Result<ScaffoldValues> {
    let ns_default = default_namespace();

    let namespace = resolve_one(
        args.namespace.clone(),
        interactive,
        "Namespace (org)?",
        &ns_default,
        true,
    )?;
    let description = resolve_one(
        args.description.clone(),
        interactive,
        "Description?",
        "",
        false,
    )?;
    let license = match args.license.clone() {
        Some(l) => l,
        None if interactive => cliclack::select("License?")
            .item(
                "MIT OR Apache-2.0".to_string(),
                "MIT OR Apache-2.0",
                "recommended",
            )
            .item("Apache-2.0".to_string(), "Apache-2.0", "")
            .item("MIT".to_string(), "MIT", "")
            .item("Elastic-2.0".to_string(), "Elastic-2.0", "")
            .interact()?,
        None => "MIT OR Apache-2.0".to_string(),
    };

    Ok(ScaffoldValues {
        name: args.name.clone(),
        namespace,
        description,
        license,
        pdk_dep: pdk_dep_line(args.pdk_path.as_deref()),
    })
}

/// A flag wins; else prompt on a TTY (default pre-filled); else use the default.
fn resolve_one(
    flag: Option<String>,
    interactive: bool,
    prompt: &str,
    default: &str,
    required: bool,
) -> std::io::Result<String> {
    match flag {
        Some(v) => Ok(v),
        None if interactive => cliclack::input(prompt)
            .default_input(default)
            .required(required)
            .interact(),
        None => Ok(default.to_string()),
    }
}

fn write_scaffold(root: &Path, values: &ScaffoldValues) -> std::io::Result<()> {
    for (rel, contents) in render_all(values) {
        let path = root.join(&rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, contents)?;
    }
    Ok(())
}
