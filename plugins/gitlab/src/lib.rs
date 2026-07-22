//! Moonlit first-party `gitlab` plugin. Calls the GitLab REST API v4 via the host
//! HTTP capability; one component instance per pipeline run holds `GitlabShared`.

mod api;
mod config;
mod context;
mod create_release;
mod related_items;
mod write_variables;
