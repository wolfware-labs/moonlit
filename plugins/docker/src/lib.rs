//! Moonlit first-party `docker` plugin. Shells out to the `docker` CLI; one
//! component instance per pipeline run holds `DockerShared` (the buildx builder
//! name recorded by `setup-buildx` and read by `build-and-push`).

mod build_and_push;
mod deploy;
mod docker;
mod login;
mod setup_buildx;
mod state;

use moonlit_sdk::prelude::*;

use build_and_push::BuildAndPush;
use deploy::Deploy;
use login::Login;
use setup_buildx::SetupBuildx;
use state::DockerShared;

moonlit_plugin! {
    name: "docker",
    state: DockerShared,
    middlewares: [Login, SetupBuildx, BuildAndPush, Deploy],
}
