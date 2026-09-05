mod cli;
mod commands;
mod input;
mod render;
mod signal;

use clap::Parser;

use cli::{Cli, Command};

#[tokio::main]
async fn main() {
    let args = Cli::parse();
    let output = args.output;
    let verbose = args.verbose;
    let code = match args.command {
        // A bare `moonlit` prints help, like any other modern CLI. `moonlit version`
        // remains the way to see the banner.
        None => {
            use clap::CommandFactory as _;
            let mut cmd = Cli::command();
            cmd.print_help().expect("writing help to stdout");
            println!();
            0
        }
        Some(Command::Version) => commands::version::run(),
        Some(Command::Run(a)) => {
            let dry = a.dry_run;
            commands::run::run(output, verbose, a, dry).await
        }
        Some(Command::Validate(a)) => commands::validate::run(output, verbose, a).await,
        Some(Command::Plugin(p)) => commands::plugin::run(output, verbose, p).await,
        Some(Command::Login(a)) => commands::login::run(a).await,
        Some(Command::Logout(a)) => commands::logout::run(a).await,
        Some(Command::Cache(c)) => commands::cache::run(output, c),
    };
    std::process::exit(code);
}
