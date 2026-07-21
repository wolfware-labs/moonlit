//! Moonlit first-party `github` plugin. Calls the GitHub REST API via the host
//! HTTP capability; one component instance per pipeline run holds `GithubShared`.

mod api;
mod config;
mod context;
