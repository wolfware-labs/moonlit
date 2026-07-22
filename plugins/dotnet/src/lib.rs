//! Moonlit first-party `dotnet` plugin. Shells out to the `dotnet` CLI for
//! build / pack / push / test. One component instance per pipeline run.

mod build;
mod config;
mod dotnet;
mod pack;
mod push;
mod test;
mod trx;
mod version;

use moonlit_plugin_sdk::prelude::*;

use build::Build;
use config::DotnetConfig;
use pack::Pack;
use push::Push;
use test::Test;

moonlit_plugin! {
    name: "dotnet",
    config: DotnetConfig,
    middlewares: [Build, Pack, Push, Test],
}
