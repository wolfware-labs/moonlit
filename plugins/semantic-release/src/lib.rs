//! Moonlit first-party `semantic-release` plugin. Pure-Rust: parses conventional
//! commits, computes the next semantic version, and produces structured changelog
//! categories. Offline except for the opt-in AI-assisted changelog refinement
//! (`ai` config block). One component instance per run holds `SrShared`.

mod ai;
mod analyze;
mod calculate_version;
mod changelog;
mod config;
mod convert;
mod generate_changelog;
mod models;
mod refine;
mod version;

use moonlit_sdk::prelude::*;

use analyze::Analyze;
use calculate_version::CalculateVersion;
use config::SrPluginConfig;
use generate_changelog::GenerateChangelog;
use models::SrShared;

moonlit_plugin! {
    name: "semantic-release",
    config: SrPluginConfig,
    state: SrShared,
    middlewares: [Analyze, CalculateVersion, GenerateChangelog],
}
