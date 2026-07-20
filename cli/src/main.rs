mod cli;
mod commands;

use clap::Parser;

use cli::{Cli, Command};

#[tokio::main]
async fn main() {
    let args = Cli::parse();
    let code = match args.command {
        None | Some(Command::Version) => commands::version::run(),
    };
    std::process::exit(code);
}
