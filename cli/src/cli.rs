//! Command-line surface: the `clap` derive tree and shared value types.

use clap::{Parser, Subcommand, ValueEnum};

/// The `moonlit` CLI. With no subcommand, behaves like `version` (C# parity).
#[derive(Debug, Parser)]
#[command(name = "moonlit", version, disable_help_subcommand = true)]
pub struct Cli {
    /// Output mode; auto-detects pretty (TTY) vs plain when omitted.
    #[arg(long, value_enum, global = true)]
    pub output: Option<OutputMode>,

    /// Verbose logging (DEBUG/TRACE).
    #[arg(short = 'v', long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputMode {
    Pretty,
    Json,
    Plain,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Print the banner, version, author, and license.
    Version,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_subcommand_parses_to_none() {
        let cli = Cli::try_parse_from(["moonlit"]).unwrap();
        assert!(cli.command.is_none());
    }

    #[test]
    fn version_subcommand_parses() {
        let cli = Cli::try_parse_from(["moonlit", "version"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Version)));
    }

    #[test]
    fn output_flag_parses_each_mode() {
        for (s, want) in [
            ("pretty", OutputMode::Pretty),
            ("json", OutputMode::Json),
            ("plain", OutputMode::Plain),
        ] {
            let cli = Cli::try_parse_from(["moonlit", "--output", s]).unwrap();
            assert_eq!(cli.output, Some(want));
        }
    }
}
