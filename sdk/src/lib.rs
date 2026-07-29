//! Moonlit plugin SDK: write a plugin as typed `Middleware` structs + one
//! `moonlit_plugin!` block. See `docs`/README for the authoring model.

/// Generated WIT bindings. Public because the `moonlit_plugin!` macro output
/// (expanded in the author crate) references this exact path via
/// `export!(Component with_types_in moonlit_sdk::bindings)`.
#[allow(clippy::too_many_arguments)]
pub mod bindings {
    wit_bindgen::generate!({
        path: "wit",
        world: "plugin",
        generate_all,
        pub_export_macro: true,
        default_bindings_module: "moonlit_sdk::bindings",
    });
}

pub use bindings::export;

pub use moonlit_sdk_macros::moonlit_plugin;

mod result;
pub use result::{MiddlewareResult, Output};

pub mod config;

mod context;
pub use context::{Context, Host, LogLevel};

#[cfg(target_arch = "wasm32")]
mod real_host;
#[cfg(target_arch = "wasm32")]
pub use real_host::RealHost;

pub mod env;

pub mod process;

pub mod state;

pub mod http;

pub mod changelog;

pub mod clock;

pub mod random;

pub mod testing;

mod middleware;
pub use middleware::Middleware;

mod plugin_config;
pub use plugin_config::PluginConfig;

pub mod prelude {
    pub use crate::config::from_json_value;
    pub use crate::moonlit_plugin;
    pub use crate::process::LineHandler;
    pub use crate::state::Shared;
    pub use crate::PluginConfig;
    pub use crate::{Context, LogLevel, Middleware, MiddlewareResult, Output};
    pub use serde::Deserialize;
}

#[cfg(test)]
mod wit_drift {
    /// The vendored canonical WIT must match the engine's source of truth when
    /// building in-repo. Inert for crates.io consumers (no sibling engine/).
    #[test]
    fn vendored_moonlit_plugin_wit_matches_engine() {
        let engine = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../engine/wit/moonlit-plugin.wit");
        if !engine.exists() {
            return; // published-crate context: nothing to compare against
        }
        let vendored =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("wit/moonlit-plugin.wit");
        assert_eq!(
            std::fs::read_to_string(&vendored).unwrap(),
            std::fs::read_to_string(&engine).unwrap(),
            "sdk/wit/moonlit-plugin.wit drifted from engine/wit/moonlit-plugin.wit; re-vendor"
        );
    }
}
