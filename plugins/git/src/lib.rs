//! Moonlit first-party `git` plugin. Shells to the `git` CLI via the host
//! process capability; one component instance per pipeline run holds `GitShared`.

mod commits;
mod latest_tag;
mod repo_context;
mod shared;
mod tag;

use moonlit_plugin_sdk::prelude::*;

use commits::Commits;
use latest_tag::LatestTag;
use repo_context::RepoContext;
use shared::GitShared;
use tag::Tag;

moonlit_plugin! {
    name: "git",
    state: GitShared,
    middlewares: [RepoContext, LatestTag, Commits, Tag],
}
