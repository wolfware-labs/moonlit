use clap::Parser;
use moonlit::cli::{Cli, Command};
use moonlit::commands;
use moonlit::commands::{run, version};
use std::process::exit;

#[tokio::main]
async fn main() {
    let args = Cli::parse();
    let output = args.output;
    let verbose = args.verbose;
    let code = match args.command {
        None => {
            use clap::CommandFactory as _;
            let mut cmd = Cli::command();
            cmd.print_help().expect("writing help to stdout");
            println!();
            0
        }
        Some(Command::Version) => version::run(),
        Some(Command::Run(a)) => run::run(output, verbose, a).await,
        Some(Command::Validate(a)) => commands::validate::run(output, verbose, a).await,
        Some(Command::Plugin(p)) => commands::plugin::run(output, verbose, p).await,
        Some(Command::Login(a)) => commands::login::run(a).await,
        Some(Command::Logout(a)) => commands::logout::run(a).await,
        Some(Command::Cache(c)) => commands::cache::run(output, c),
    };
    exit(code);
}
