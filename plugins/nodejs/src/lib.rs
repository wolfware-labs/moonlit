//! Moonlit first-party `nodejs` plugin. Shells out to the `npm` CLI for
//! install / run-script / build / pack / push / test. One component instance per run.

mod build;
mod config;
mod install;
mod npm;
mod pack;
mod push;
mod run_script;
mod test;

use moonlit_sdk::prelude::*;

use build::Build;
use config::NodeConfig;
use install::Install;
use pack::Pack;
use push::Push;
use run_script::RunScript;
use test::Test;

moonlit_plugin! {
    name: "nodejs",
    config: NodeConfig,
    middlewares: [Install, RunScript, Build, Pack, Push, Test],
}
