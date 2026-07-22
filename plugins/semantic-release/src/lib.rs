//! Moonlit first-party `semantic-release` plugin. Pure-Rust, fully offline: parses
//! conventional commits, computes the next semantic version, and produces structured
//! changelog categories. One component instance per run holds `SrShared`.

mod analyze;
mod calculate_version;
mod convert;
mod models;
mod version;
