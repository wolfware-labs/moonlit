//! Moonlit first-party `git` plugin. Shells to the `git` CLI via the host
//! process capability; one component instance per pipeline run holds `GitShared`.

mod commits;
mod latest_tag;
mod push;
mod repo_context;
mod shared;
mod tag;

use moonlit_sdk::prelude::*;

use commits::Commits;
use latest_tag::LatestTag;
use push::Push;
use repo_context::RepoContext;
use shared::GitShared;
use tag::Tag;

moonlit_plugin! {
    name: "git",
    state: GitShared,
    middlewares: [RepoContext, LatestTag, Commits, Tag, Push],
}
