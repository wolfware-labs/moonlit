//! Moonlit first-party `git` plugin. Shells to the `git` CLI via the host
//! process capability; one component instance per pipeline run holds `GitShared`.

mod latest_tag;
mod repo_context;
mod shared;

use moonlit_plugin_sdk::prelude::*;

use latest_tag::LatestTag;
use repo_context::RepoContext;
use shared::GitShared;

moonlit_plugin! {
    name: "git",
    state: GitShared,
    middlewares: [RepoContext, LatestTag],
}
