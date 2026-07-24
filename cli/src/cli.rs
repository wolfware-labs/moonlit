//! Command-line surface: the `clap` derive tree and shared value types.

use std::path::PathBuf;
use std::time::Duration;

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
    /// Run a release pipeline.
    Run(RunArgs),
    /// Alias of `run` (docs compatibility).
    Release(RunArgs),
    /// Parse, resolve plugins, and verify middleware refs without executing.
    Validate(ValidateArgs),
    /// Scaffold, build, and inspect plugins.
    #[command(subcommand)]
    Plugin(PluginCommand),
    /// Print the banner, version, author, and license.
    Version,
}

#[derive(Debug, Subcommand)]
pub enum PluginCommand {
    /// Print a component's metadata and middlewares.
    Inspect(PluginInspectArgs),
    /// Scaffold a new plugin crate.
    New(PluginNewArgs),
    /// Build the plugin in the current directory to a WASI-P2 component.
    Build(PluginBuildArgs),
    /// Publish a built component to an OCI registry.
    Publish(PluginPublishArgs),
}

#[derive(Debug, clap::Args)]
pub struct PluginInspectArgs {
    /// Path to a built `.wasm` component.
    pub path: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct PluginNewArgs {
    /// Crate name for the new plugin (also the directory created).
    pub name: String,
    /// Publish namespace (org). Prompted on a TTY; defaults to git user or "my-org".
    #[arg(long)]
    pub namespace: Option<String>,
    /// One-line description. Prompted on a TTY; defaults to empty.
    #[arg(long)]
    pub description: Option<String>,
    /// SPDX license. Prompted on a TTY; defaults to "Apache-2.0".
    #[arg(long)]
    pub license: Option<String>,
    /// Emit a `path = …` SDK dependency (local dev) instead of a crates.io version.
    #[arg(long)]
    pub sdk_path: Option<PathBuf>,
}

#[derive(Debug, clap::Args)]
pub struct PluginBuildArgs {
    /// Build in release mode (optimized, smaller component).
    #[arg(long)]
    pub release: bool,
    /// Directory of the plugin crate (default: current directory).
    #[arg(long)]
    pub manifest_path: Option<PathBuf>,
}

#[derive(Debug, clap::Args)]
pub struct PluginPublishArgs {
    /// Target reference, e.g. `oci://ghcr.io/acme/plugin:1.0.0` or `ghcr.io/acme/plugin:1.0.0`.
    #[arg(value_name = "REF")]
    pub reference: String,
    /// Component file to publish (default: the crate's release build output).
    #[arg(long)]
    pub file: Option<PathBuf>,
    /// Directory of the plugin crate (default: current directory).
    #[arg(long)]
    pub manifest_path: Option<PathBuf>,
}

#[derive(Debug, clap::Args)]
pub struct RunArgs {
    /// Pipeline file (default: release.yml, then moonlit.yml).
    #[arg(short = 'f', long = "file")]
    pub file: Option<PathBuf>,

    /// Working directory (default: current).
    #[arg(short = 'w', visible_short_alias = 'd', long = "working-dir")]
    pub working_dir: Option<PathBuf>,

    /// Stage(s) to run; repeatable and comma-separated.
    #[arg(short = 's', long = "stage", value_delimiter = ',')]
    pub stages: Vec<String>,

    /// Pipeline argument(s), `key=value`; repeatable.
    #[arg(short = 'a', long = "arg", value_parser = parse_kv)]
    pub args: Vec<(String, String)>,

    /// Fail instead of pulling on a cache miss.
    #[arg(long)]
    pub offline: bool,

    /// Per-step timeout (e.g. `300s`, `1m30s`).
    #[arg(long = "step-timeout", value_parser = parse_step_timeout)]
    pub step_timeout: Option<Duration>,

    /// Load and validate only; do not execute.
    #[arg(long = "dry-run")]
    pub dry_run: bool,
}

#[derive(Debug, clap::Args)]
pub struct ValidateArgs {
    #[arg(short = 'f', long = "file")]
    pub file: Option<PathBuf>,
    #[arg(short = 'w', visible_short_alias = 'd', long = "working-dir")]
    pub working_dir: Option<PathBuf>,
}

/// Parse a `key=value` argument (split on the first `=`).
pub fn parse_kv(s: &str) -> Result<(String, String), String> {
    match s.split_once('=') {
        Some((k, v)) if !k.is_empty() => Ok((k.to_string(), v.to_string())),
        _ => Err(format!("expected key=value, got '{s}'")),
    }
}

fn parse_step_timeout(s: &str) -> Result<Duration, String> {
    humantime::parse_duration(s).map_err(|e| e.to_string())
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

    #[test]
    fn working_dir_short_alias_d_matches_w() {
        let a = Cli::try_parse_from(["moonlit", "run", "-w", "/x"]).unwrap();
        let b = Cli::try_parse_from(["moonlit", "run", "-d", "/x"]).unwrap();
        let wd = |c: &Cli| match &c.command {
            Some(Command::Run(r)) => r.working_dir.clone(),
            _ => None,
        };
        assert_eq!(wd(&a), Some(PathBuf::from("/x")));
        assert_eq!(wd(&a), wd(&b));
    }

    #[test]
    fn stages_accept_repeat_and_comma() {
        let c = Cli::try_parse_from(["moonlit", "run", "-s", "a,b", "-s", "c"]).unwrap();
        let stages = match c.command {
            Some(Command::Run(r)) => r.stages,
            _ => vec![],
        };
        assert_eq!(stages, vec!["a", "b", "c"]);
    }

    #[test]
    fn parse_kv_splits_first_equals() {
        assert_eq!(parse_kv("k=v=w"), Ok(("k".into(), "v=w".into())));
        assert!(parse_kv("noeq").is_err());
        assert!(parse_kv("=v").is_err());
    }

    #[test]
    fn step_timeout_parses_humantime() {
        let c = Cli::try_parse_from(["moonlit", "run", "--step-timeout", "90s"]).unwrap();
        let t = match c.command {
            Some(Command::Run(r)) => r.step_timeout,
            _ => None,
        };
        assert_eq!(t, Some(Duration::from_secs(90)));
    }
}
