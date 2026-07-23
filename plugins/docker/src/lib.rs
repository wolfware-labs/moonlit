//! Moonlit first-party `docker` plugin. Shells out to the `docker` CLI; one
//! component instance per pipeline run holds `DockerShared` (the buildx builder
//! name). `moonlit_plugin!` wiring is added once every middleware exists.

mod build_and_push;
mod deploy;
mod docker;
mod login;
mod setup_buildx;
mod state;
