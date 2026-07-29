mod run_modules;

use moonlit_sdk::prelude::*;

use run_modules::RunModules;

moonlit_plugin! {
    name: "moonlit",
    middlewares: [RunModules],
}
