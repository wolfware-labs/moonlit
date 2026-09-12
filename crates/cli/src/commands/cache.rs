//! `moonlit cache [ls|clean]` — inspect or clear the content cache (`<OS cache dir>/moonlit`).

use moonlit_engine::cache::Cache;

use crate::cli::{CacheCommand, OutputMode};
use crate::render::resolve_mode;

pub fn run(output: Option<OutputMode>, cmd: CacheCommand) -> i32 {
    let cache = match Cache::new() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };
    match cmd {
        CacheCommand::Ls => ls(output, &cache),
        CacheCommand::Clean => clean(&cache),
    }
}

fn ls(output: Option<OutputMode>, cache: &Cache) -> i32 {
    let items = cache.list();
    let stdout_tty = std::io::IsTerminal::is_terminal(&std::io::stdout());
    match resolve_mode(output, stdout_tty) {
        OutputMode::Json => {
            let arr: Vec<_> = items
                .iter()
                .map(|(_key, m)| {
                    serde_json::json!({
                        "source": m.source,
                        "digest": m.digest,
                        "size": m.size,
                        "pulledAt": m.pulled_at,
                        "middlewares": m.middlewares,
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&arr).unwrap());
        }
        OutputMode::Plain => {
            if items.is_empty() {
                println!("cache is empty");
            }
            for (_key, m) in &items {
                let digest = m.digest.as_deref().unwrap_or("-");
                let mw = m.middlewares.as_ref().map(|v| v.len()).unwrap_or(0);
                println!(
                    "{}  {}  {} bytes  {} middlewares",
                    m.source, digest, m.size, mw
                );
            }
        }
        OutputMode::Pretty => {
            if items.is_empty() {
                println!("cache is empty");
                return 0;
            }
            use comfy_table::{Table, presets::UTF8_BORDERS_ONLY};
            let mut table = Table::new();
            table.load_preset(UTF8_BORDERS_ONLY);
            table.set_header(["Reference", "Digest", "Size", "Middlewares"]);
            for (_key, m) in &items {
                let digest = m.digest.as_deref().unwrap_or("-");
                let short = digest.get(..19).unwrap_or(digest);
                let mw = m.middlewares.as_ref().map(|v| v.len()).unwrap_or(0);
                table.add_row([
                    m.source.clone(),
                    short.to_string(),
                    format!("{} B", m.size),
                    mw.to_string(),
                ]);
            }
            println!("{table}");
        }
    }
    0
}

fn clean(cache: &Cache) -> i32 {
    match cache.clean() {
        Ok(stats) => {
            println!(
                "Removed {} plugins, {} blobs, {} refs; freed {} bytes.",
                stats.plugins, stats.blobs, stats.refs, stats.bytes
            );
            0
        }
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}
