//! Moonlit first-party `semantic-release` plugin. Pure-Rust, fully offline: parses
//! conventional commits, computes the next semantic version, and produces structured
//! changelog categories. One component instance per run holds `SrShared`.

mod analyze;
mod calculate_version;
mod changelog;
mod convert;
mod generate_changelog;
mod models;
mod version;

use moonlit_plugin_sdk::prelude::*;

use analyze::Analyze;
use calculate_version::CalculateVersion;
use generate_changelog::GenerateChangelog;
use models::SrShared;

moonlit_plugin! {
    name: "semantic-release",
    state: SrShared,
    middlewares: [Analyze, CalculateVersion, GenerateChangelog],
}
