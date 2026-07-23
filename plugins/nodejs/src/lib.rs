//! Moonlit first-party `nodejs` plugin. Shells out to the `npm` CLI for
//! install / run-script / build / pack / push / test. One component instance per run.

mod build;
mod config;
mod install;
mod npm;
mod pack;
mod run_script;
