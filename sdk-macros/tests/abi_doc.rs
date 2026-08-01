//! The crate doc names the plugin ABI this macro emits, and that string is the docs.rs front page
//! for `moonlit-sdk-macros`. Nothing pinned it to reality, so it survived two ABI bumps unchanged
//! (0.1.0 -> 0.2.0 in `cd22a17`, 0.2.0 -> 0.3.0 in `8bac9d5`) while claiming the old version.
//!
//! Tie it to the WIT package that actually ships.

/// The `package …;` line of the canonical WIT, e.g. `moonlit:plugin@0.3.0`.
/// `None` in a published-crate context, where there is no sibling `sdk/` to read.
fn shipped_abi() -> Option<String> {
    let wit =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../sdk/wit/moonlit-plugin.wit");
    let text = std::fs::read_to_string(wit).ok()?;
    let package = text
        .lines()
        .find_map(|l| l.trim().strip_prefix("package "))?
        .trim()
        .trim_end_matches(';')
        .to_string();
    Some(package)
}

/// The `//!` header block at the top of this crate's `lib.rs`.
fn crate_doc() -> String {
    include_str!("../src/lib.rs")
        .lines()
        .take_while(|l| l.starts_with("//!"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn crate_doc_states_the_shipped_abi_version() {
    let Some(abi) = shipped_abi() else {
        return; // published-crate context: nothing to compare against
    };
    let doc = crate_doc();
    assert!(
        doc.contains(&abi),
        "sdk-macros' crate doc must name the shipped ABI `{abi}`, since it is the docs.rs front \
         page. It currently reads:\n{doc}"
    );
}
