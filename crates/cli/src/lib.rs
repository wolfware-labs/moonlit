//! The `moonlit` CLI, as a library.
//!
//! `main.rs` is a thin entry point over this crate: it parses arguments and
//! dispatches to `commands`. Keeping the modules here rather than in the binary
//! means they can be exercised directly by unit and integration tests.

pub mod cli;
pub mod commands;
pub mod input;
pub mod render;
pub mod signal;
