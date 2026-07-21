//! Moonlit first-party `github` plugin. Calls the GitHub REST API via the host
//! HTTP capability; one component instance per pipeline run holds `GithubShared`.

mod api;
mod config;
mod context;
mod create_release;
mod related_items;
mod write_variables;

use moonlit_plugin_sdk::prelude::*;

use config::GithubPluginConfig;
use context::GithubShared;
use create_release::CreateRelease;
use related_items::RelatedItems;
use write_variables::WriteVariables;

moonlit_plugin! {
    name: "github",
    config: GithubPluginConfig,
    state: GithubShared,
    middlewares: [RelatedItems, CreateRelease, WriteVariables],
}
