//! Structural verification of the canonical `moonlit:plugin` WIT contract.
//!
//! Resolving `engine/wit/` is the primary gate: it fails loudly if the contract is
//! malformed or a vendored WASI dependency is missing. The assertions then pin the
//! contract's shape so a later edit cannot silently drop or rename an export.

use wit_parser::{PackageId, Resolve, TypeDefKind, WorldId, WorldItem, WorldKey};

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
fn package_is_moonlit_plugin_0_3_0() {
    let (resolve, package_id, _) = load();
    let name = &resolve.packages[package_id].name;
    assert_eq!(name.namespace, "moonlit");
    assert_eq!(name.name, "plugin");
    assert_eq!(
        name.version.as_ref().map(ToString::to_string).as_deref(),
        Some("0.3.0")
    );
}

/// `PLUGIN_WORLD` is stamped into every published OCI config as `moonlit.world`, and the registry
/// rejects any manifest whose value is not exactly its own `SupportedWorld`. Nothing tied the
/// constant to the WIT it claims to describe, which is how it sat two revisions behind: the 0.2.0
/// bump (`b5e8543`) and the 0.3.0 bump (`8bac9d5`) both changed the contract and left it at 0.1.0.
/// Derive the expectation from the resolved package so the next bump cannot miss it.
#[test]
fn plugin_world_constant_matches_the_shipped_wit() {
    let (resolve, package_id, _) = load();
    let name = &resolve.packages[package_id].name;
    let expected = format!(
        "{}:{}@{}",
        name.namespace,
        name.name,
        name.version
            .as_ref()
            .expect("the WIT package declares a version")
    );
    assert_eq!(
        moonlit_engine::publish::PLUGIN_WORLD,
        expected,
        "PLUGIN_WORLD must name the WIT this engine ships. Bumping the contract means bumping this \
         constant AND the registry's PublishService.SupportedWorld in the same window - the \
         registry rejects any other value with ManifestInvalid."
    );
}

/// Names of the fields declared by a `record` type in the `types` interface.
fn record_field_names<'a>(
    resolve: &'a Resolve,
    package_id: PackageId,
    record: &str,
) -> Vec<&'a str> {
    let types_id = resolve.packages[package_id].interfaces["types"];
    let type_id = resolve.interfaces[types_id].types[record];
    match &resolve.types[type_id].kind {
        TypeDefKind::Record(rec) => rec.fields.iter().map(|f| f.name.as_str()).collect(),
        other => panic!("`{record}` must be a record, found {other:?}"),
    }
}

#[test]
fn plugin_metadata_carries_optional_icon() {
    let (resolve, package_id, _) = load();
    let fields = record_field_names(&resolve, package_id, "plugin-metadata");
    assert!(
        fields.contains(&"icon"),
        "plugin-metadata must declare `icon`; fields = {fields:?}"
    );
}

#[test]
fn middleware_info_carries_optional_input_and_output_schema() {
    let (resolve, package_id, _) = load();
    let fields = record_field_names(&resolve, package_id, "middleware-info");
    assert!(
        fields.contains(&"input-schema"),
        "middleware-info must declare `input-schema`; fields = {fields:?}"
    );
    assert!(
        fields.contains(&"output-schema"),
        "middleware-info must declare `output-schema`; fields = {fields:?}"
    );
}

#[test]
fn world_exports_exactly_the_four_entrypoints() {
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
    assert_eq!(exports, ["describe", "execute", "init", "list-middlewares"]);
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
