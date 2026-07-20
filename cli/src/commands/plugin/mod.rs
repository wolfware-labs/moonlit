//! `moonlit plugin` — author-facing plugin lifecycle commands.

mod inspect;
// Pure scaffold logic for the upcoming `plugin new` command; not yet wired
// into `run`, so its items are unused until that task lands.
#[allow(dead_code)]
mod scaffold;
#[allow(dead_code)]
mod templates;
mod wasm;

use crate::cli::{OutputMode, PluginCommand};

pub async fn run(output: Option<OutputMode>, _verbose: bool, cmd: PluginCommand) -> i32 {
    match cmd {
        PluginCommand::Inspect(args) => inspect::run(output, args).await,
    }
}
