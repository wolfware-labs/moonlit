//! Structural verification of the canonical `moonlit:plugin` WIT contract.
//!
//! Resolving `engine/wit/` is the primary gate: it fails loudly if the contract is
//! malformed or a vendored WASI dependency is missing. The assertions then pin the
//! contract's shape so a later edit cannot silently drop or rename an export.

use wit_parser::{PackageId, Resolve, WorldId, WorldItem, WorldKey};

fn load() -> (Resolve, PackageId, WorldId) {
    let wit_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/wit");
    let mut resolve = Resolve::new();
    let (package_id, _paths) = resolve
        .push_dir(wit_dir)
        .expect("engine/wit must resolve (contract valid + WASI deps vendored)");
    let world_id = *resolve.packages[package_id]
        .worlds
        .get("plugin")
        .expect("package must define a `plugin` world");
    (resolve, package_id, world_id)
}

#[test]
fn package_is_moonlit_plugin_0_1_0() {
    let (resolve, package_id, _) = load();
    let name = &resolve.packages[package_id].name;
    assert_eq!(name.namespace, "moonlit");
    assert_eq!(name.name, "plugin");
    assert_eq!(
        name.version.as_ref().map(ToString::to_string).as_deref(),
        Some("0.1.0")
    );
}

#[test]
fn world_exports_exactly_the_three_entrypoints() {
    let (resolve, _, world_id) = load();
    let world = &resolve.worlds[world_id];
    let mut exports: Vec<&str> = world
        .exports
        .iter()
        .filter_map(|(key, item)| match (key, item) {
            (WorldKey::Name(name), WorldItem::Function(_)) => Some(name.as_str()),
            _ => None,
        })
        .collect();
    exports.sort_unstable();
    assert_eq!(exports, ["execute", "init", "list-middlewares"]);
}

#[test]
fn types_interface_declares_the_abi_types() {
    let (resolve, package_id, _) = load();
    let types_id = resolve.packages[package_id].interfaces["types"];
    let types = &resolve.interfaces[types_id];
    for name in [
        "json-value",
        "log-level",
        "release-context",
        "middleware-result",
        "middleware-info",
        "plugin-metadata",
    ] {
        assert!(types.types.contains_key(name), "types is missing `{name}`");
    }
}

#[test]
fn host_and_process_interfaces_expose_expected_items() {
    let (resolve, package_id, _) = load();
    let pkg = &resolve.packages[package_id];

    let host = &resolve.interfaces[pkg.interfaces["host"]];
    for f in ["log", "get-config", "report-progress"] {
        assert!(host.functions.contains_key(f), "host is missing `{f}`");
    }

    let process = &resolve.interfaces[pkg.interfaces["process"]];
    for f in ["spawn", "run"] {
        assert!(
            process.functions.contains_key(f),
            "process is missing `{f}`"
        );
    }
    assert!(
        process.types.contains_key("child"),
        "process is missing the `child` resource"
    );
}

#[test]
fn world_imports_the_five_wasi_0_2_3_interfaces() {
    let (resolve, _, world_id) = load();
    let world = &resolve.worlds[world_id];
    let mut wasi: Vec<String> = world
        .imports
        .values()
        .filter_map(|item| match item {
            WorldItem::Interface { id, .. } => {
                let iface = &resolve.interfaces[*id];
                let pkg = &resolve.packages[iface.package?];
                (pkg.name.namespace == "wasi").then(|| {
                    format!(
                        "{}@{}",
                        pkg.name.name,
                        pkg.name
                            .version
                            .as_ref()
                            .map(ToString::to_string)
                            .unwrap_or_default()
                    )
                })
            }
            _ => None,
        })
        .collect();
    wasi.sort_unstable();
    wasi.dedup();
    for expected in [
        "cli@0.2.3",
        "clocks@0.2.3",
        "filesystem@0.2.3",
        "http@0.2.3",
        "random@0.2.3",
    ] {
        assert!(
            wasi.contains(&expected.to_string()),
            "world must import wasi:{expected}; imports = {wasi:?}"
        );
    }
}
