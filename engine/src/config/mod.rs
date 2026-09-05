//! Pipeline configuration parsing (§4): YAML → validated [`PipelineConfig`] or [`ConfigDiagnostic`].
//!
//! Four hand-rolled stages over the `saphyr-parser` event stream: parse → convert → cleanup →
//! validate. No `$()` substitution, layering, coercion, or conditions — those are Phase 3;
//! `$(...)` text is preserved verbatim as raw strings.

// The spec (§7.2) fixes the fallible surface as `Result<_, ConfigDiagnostic>` (unboxed).
// `ConfigDiagnostic` carries the source text + labels and so trips clippy's
// `result_large_err`; boxing it would contradict the mandated signature and add indirection
// on the cold error path. We deliberately keep the unboxed error and silence the lint here.
#![allow(clippy::result_large_err)]

pub mod diagnostic;
pub mod model;

mod cleanup;
mod convert;
mod tree;
mod validate;

pub use diagnostic::ConfigDiagnostic;
pub use model::PipelineConfig;

use diagnostic::Source;

/// Parse a Moonlit pipeline configuration file.
///
/// `source_name` labels the source in diagnostics (e.g. `release.yml`). Runs parse → convert →
/// cleanup → validate, short-circuiting to a [`ConfigDiagnostic`] on the first failure.
pub fn parse_config(yaml: &str, source_name: &str) -> Result<PipelineConfig, ConfigDiagnostic> {
    let src = Source::new(yaml, source_name);
    let tree = tree::build_tree(&src)?;
    let config = convert::convert(tree, &src)?;
    let config = cleanup::cleanup(config);
    validate::validate(&config, &src)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = "\
name: demo
plugins:
  - name: git
    url: oci://reg.example.com/wolfware/git:2.0.0
stages:
  build:
    - name: tag
      run: git.tag
";

    #[test]
    fn end_to_end_happy_path() {
        let c = parse_config(GOOD, "release.yml").expect("valid");
        assert_eq!(c.name, "demo");
        assert_eq!(c.plugins.value.len(), 1);
        assert_eq!(c.stages.value[0].steps[0].run.value.middleware, "tag");
    }

    #[test]
    fn end_to_end_trims_name() {
        let c = parse_config("name: '  spaced  '\nplugins:\n  - name: p\n    url: file:///p.wasm\nstages:\n  s:\n    - name: a\n      run: p.x\n", "release.yml").unwrap();
        assert_eq!(c.name, "spaced");
    }

    #[test]
    fn end_to_end_no_stages_error() {
        let err = parse_config(
            "plugins:\n  - name: p\n    url: file:///p.wasm\n",
            "release.yml",
        )
        .unwrap_err();
        assert_eq!(
            err.message(),
            "No stages defined. A pipeline needs at least one stage."
        );
    }

    #[test]
    fn end_to_end_no_plugins_error() {
        let err = parse_config(
            "stages:\n  s:\n    - name: a\n      run: p.x\n",
            "release.yml",
        )
        .unwrap_err();
        assert_eq!(
            err.message(),
            "No plugins declared. Every step runs a middleware from a plugin, so at least one is required."
        );
    }

    #[test]
    fn end_to_end_non_ascii_key_before_error_has_correct_byte_span() {
        // The `café` stage name is a non-ASCII map key (verbatim, not schema-matched) whose
        // extra UTF-8 byte (`é` = 2 bytes, 1 char) precedes the bad `run:` value below it. If
        // the parser's char-index spans were used as byte offsets directly (instead of being
        // mapped through `char_to_byte`, see `tree.rs`), the diagnostic's span would land one
        // byte short and this substring check would fail.
        let yaml = "plugins:\n  - name: p\n    url: file:///p.wasm\nstages:\n  café:\n    - name: a\n      run: nodot\n";
        let err = parse_config(yaml, "release.yml").unwrap_err();
        assert_eq!(
            err.message(),
            "'nodot' is not a valid run reference; use the format 'plugin.middleware'."
        );
        let span = err.span().expect("has a span");
        let (start, len) = (span.offset(), span.len());
        assert_eq!(&yaml[start..start + len], "nodot");
    }
}
