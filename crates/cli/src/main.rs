use clap::Parser;
use moonlit::cli::{Cli, Command};
use moonlit::commands::auth::{login, logout};
use moonlit::commands::{add, cache, plugin, run, search, update, validate, version};
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
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
    Some(Command::Validate(a)) => validate::run(output, verbose, a).await,
    Some(Command::Plugin(p)) => plugin::run(output, verbose, p).await,
    Some(Command::Login(a)) => login::run(a).await,
    Some(Command::Logout(a)) => logout::run(a).await,
    Some(Command::Cache(c)) => cache::run(output, &c),
    Some(Command::Search(a)) => search::run(output, a).await,
    Some(Command::Add(a)) => add::run(output, a).await,
    Some(Command::Update(a)) => update::run(output, a).await,
  };
  u8::try_from(code).map_or(ExitCode::FAILURE, ExitCode::from)
}
