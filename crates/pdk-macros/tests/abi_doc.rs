//! The crate doc names the plugin ABI this macro emits, and that string is the docs.rs front page
//! for `moonlit-pdk-macros`. Nothing pinned it to reality, so it survived two ABI bumps unchanged
//! (0.1.0 -> 0.2.0 in `cd22a17`, 0.2.0 -> 0.3.0 in `8bac9d5`) while claiming the old version.
//!
//! Tie it to the WIT package that actually ships.

/// The `package …;` line of the canonical WIT, e.g. `moonlit:plugin@0.4.0`.
fn shipped_abi() -> String {
  let wit = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../pdk/wit/moonlit-plugin.wit");
  let text = std::fs::read_to_string(&wit).unwrap_or_else(|e| panic!("cannot read the shipped WIT at {}: {e}", wit.display()));
  text
    .lines()
    .find_map(|l| l.trim().strip_prefix("package "))
    .expect("the shipped WIT declares a package")
    .trim()
    .trim_end_matches(';')
    .to_string()
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
  let abi = shipped_abi();
  let doc = crate_doc();
  assert!(
    doc.contains(&abi),
    "moonlit-pdk-macros' crate doc must name the shipped ABI `{abi}`, since it is the \
         docs.rs front page. It currently reads:\n{doc}"
  );
}
