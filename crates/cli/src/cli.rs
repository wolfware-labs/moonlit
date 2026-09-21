use std::path::PathBuf;
use std::time::Duration;

use clap::{Parser, Subcommand, ValueEnum};

/// The `moonlit` CLI.
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
    /// Parse, resolve plugins, and verify middleware refs without executing.
    Validate(ValidateArgs),
    /// Scaffold, build, and inspect plugins.
    #[command(subcommand)]
    Plugin(PluginCommand),
    /// Print the banner, version, author, and license.
    Version,
    /// Sign in to an OCI registry in the browser, or with `--token` for CI.
    Login(LoginArgs),
    /// Remove the stored credential for an OCI registry, revoking the token server-side if there is one.
    Logout(LogoutArgs),
    /// Inspect or clear the plugin content cache.
    #[command(subcommand)]
    Cache(CacheCommand),
}

#[derive(Debug, Subcommand)]
pub enum CacheCommand {
    /// List cached plugins.
    Ls,
    /// Remove all cached content.
    Clean,
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
    /// Path to a built `.wasm` component, or a plugin ref (`oci://…`, `file://…`, `http(s)://…`).
    #[arg(value_name = "PATH|REF")]
    pub target: String,
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
    pub pdk_path: Option<PathBuf>,
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
    /// Pipeline file (default: release.yml, then release.yaml).
    #[arg(short = 'f', long = "file")]
    pub file: Option<String>,

    /// Working directory (default: current).
    #[arg(short = 'w', long = "working-dir")]
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

/// Default registry when no host is given (mirrors `gh` defaulting to github.com).
pub const DEFAULT_REGISTRY_HOST: &str = "registry.moonlit.rs";

#[derive(Debug, clap::Args)]
pub struct LoginArgs {
    /// Registry host, e.g. `registry.moonlit.rs` or `localhost:5185`.
    /// Defaults to `registry.moonlit.rs` when omitted.
    pub host: Option<String>,
    /// Registry username (Basic auth). Supplying this or `--token` selects the manual path and
    /// bypasses the browser device flow (for CI); on a TTY that path then prompts for whichever
    /// of the two you left out, and a blank username stores a token-only (Bearer) credential.
    #[arg(long)]
    pub username: Option<String>,
    /// Registry token or password. Bypasses the browser device flow (for CI).
    #[arg(long)]
    pub token: Option<String>,
}

#[derive(Debug, clap::Args)]
pub struct LogoutArgs {
    /// Registry host. Defaults to `registry.moonlit.rs` when omitted.
    pub host: Option<String>,
    /// Only remove the local credential; do not contact the server to revoke the token.
    #[arg(long)]
    pub local: bool,
}

#[derive(Debug, clap::Args)]
pub struct ValidateArgs {
    /// Pipeline file (default: release.yml, then release.yaml).
    #[arg(short = 'f', long = "file")]
    pub file: Option<PathBuf>,

    /// Working directory (default: current).
    #[arg(short = 'w', long = "working-dir")]
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
