//! Moonlit first-party `dotnet` plugin. Shells out to the `dotnet` CLI for
//! build / pack / push / test. One component instance per pipeline run.

mod build;
mod config;
mod dotnet;
mod pack;
mod version;
