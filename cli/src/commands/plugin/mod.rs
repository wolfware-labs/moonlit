//! `moonlit plugin` — author-facing plugin lifecycle commands.

mod inspect;
mod new;
mod scaffold;
mod templates;
mod wasm;

use crate::cli::{OutputMode, PluginCommand};

pub async fn run(output: Option<OutputMode>, _verbose: bool, cmd: PluginCommand) -> i32 {
    match cmd {
        PluginCommand::Inspect(args) => inspect::run(output, args).await,
        PluginCommand::New(args) => new::run(args),
    }
}
