//! Moonlit plugin SDK: write a plugin as typed `Middleware` structs + one
//! `moonlit_plugin!` block. See `docs`/README for the authoring model.

/// Generated WIT bindings. Public because the `moonlit_plugin!` macro output
/// (expanded in the author crate) references this exact path via
/// `export!(Component with_types_in moonlit_plugin_sdk::bindings)`.
#[allow(clippy::too_many_arguments)]
pub mod bindings {
    wit_bindgen::generate!({
        path: "wit",
        world: "plugin",
        generate_all,
        pub_export_macro: true,
        default_bindings_module: "moonlit_plugin_sdk::bindings",
    });
}

pub use bindings::export;

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
