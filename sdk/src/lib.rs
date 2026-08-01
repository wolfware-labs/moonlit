//! Moonlit plugin SDK: write a plugin as typed `Middleware` structs + one
//! `moonlit_plugin!` block. See `docs`/README for the authoring model.
//!
//! Targets plugin ABI `moonlit:plugin@0.3.0`: plugins carry an optional icon and
//! each middleware a JSON Schema for its typed `Input` and `Output`
//! (`Middleware::Input` / `Middleware::Output: JsonSchema`).

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
pub use result::MiddlewareResult;

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

/// Serialize a middleware `Input`/`Output` JSON Schema (draft 2020-12) to text.
///
/// The `moonlit_plugin!` macro calls this to fill `middleware-info.input-schema`
/// and `output-schema`, keeping the `schemars`/`serde_json` calls inside the SDK
/// rather than each author crate. Hidden from docs — it is macro-internal API.
#[doc(hidden)]
pub fn __schema_json<T: schemars::JsonSchema>() -> String {
    serde_json::to_string(&schemars::schema_for!(T)).unwrap_or_default()
}

/// Marker for a middleware that reads no configuration. Deserializes from `{}`
/// (or an absent config block) and yields an empty-object input schema
/// ("no declared inputs"). Input-only: it has no `Serialize` impl.
#[derive(serde::Deserialize, Default, schemars::JsonSchema)]
pub struct NoInput {}

/// Marker for a middleware that publishes no outputs. Serializes to `{}` and
/// yields an empty-object output schema ("no declared outputs").
#[derive(serde::Serialize, schemars::JsonSchema)]
pub struct NoOutput {}

mod plugin_config;
pub use plugin_config::PluginConfig;

pub mod prelude {
    pub use crate::config::from_json_value;
    pub use crate::moonlit_plugin;
    pub use crate::process::LineHandler;
    pub use crate::state::Shared;
    pub use crate::PluginConfig;
    pub use crate::{Context, LogLevel, Middleware, MiddlewareResult, NoInput, NoOutput};
    pub use serde::{Deserialize, Serialize};
}

#[cfg(test)]
mod markers {
    use super::NoInput;

    /// Pins the direction claimed by `NoInput`'s doc: it binds *from* an empty object. The
    /// `NoOutput` counterpart is covered by `result::tests::no_output_serializes_to_empty`.
    #[test]
    fn no_input_deserializes_from_an_empty_object() {
        serde_json::from_str::<NoInput>("{}").expect("NoInput binds from an empty object");
    }
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

    /// This crate's doc header names the ABI it targets, and that line is the docs.rs front page.
    /// `moonlit-sdk-macros` carried a stale version across two bumps for want of this check, so
    /// pin the string to the WIT that actually ships.
    #[test]
    fn crate_doc_states_the_shipped_abi_version() {
        let wit = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("wit/moonlit-plugin.wit");
        let text = std::fs::read_to_string(wit).unwrap();
        let abi = text
            .lines()
            .find_map(|l| l.trim().strip_prefix("package "))
            .expect("WIT declares a package")
            .trim()
            .trim_end_matches(';');

        let doc: String = include_str!("lib.rs")
            .lines()
            .take_while(|l| l.starts_with("//!"))
            .collect::<Vec<_>>()
            .join("\n");

        assert!(
            doc.contains(abi),
            "moonlit-sdk's crate doc must name the shipped ABI `{abi}`. It currently reads:\n{doc}"
        );
    }
}
