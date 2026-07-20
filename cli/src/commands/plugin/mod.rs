//! `moonlit plugin` — author-facing plugin lifecycle commands.

mod inspect;
mod wasm;

use crate::cli::{OutputMode, PluginCommand};

pub async fn run(output: Option<OutputMode>, _verbose: bool, cmd: PluginCommand) -> i32 {
    match cmd {
        PluginCommand::Inspect(args) => inspect::run(output, args).await,
    }
}
