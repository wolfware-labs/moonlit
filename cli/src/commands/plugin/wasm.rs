//! Detect and validate WASI-P2 components (used by `inspect` and `build`).

use wasmparser::{Encoding, Parser, Payload, Validator, WasmFeatures};

/// True if `bytes` is a component (not a core module). Err if not a wasm binary.
// The header (magic + version) is always the first payload of a valid binary,
// so this loop only ever runs once; suppress clippy's structural lint for it.
#[allow(clippy::never_loop)]
pub fn is_component(bytes: &[u8]) -> Result<bool, String> {
    for payload in Parser::new(0).parse_all(bytes) {
        match payload.map_err(|e| format!("not a valid wasm binary: {e}"))? {
            Payload::Version { encoding, .. } => return Ok(encoding == Encoding::Component),
            _ => break,
        }
    }
    Err("empty or headerless wasm binary".to_string())
}

/// Full structural validation; catches truncated or malformed components.
pub fn validate(bytes: &[u8]) -> Result<(), String> {
    let mut v = Validator::new_with_features(WasmFeatures::all());
    v.validate_all(bytes)
        .map(|_types| ())
        .map_err(|e| format!("invalid component: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    // The smallest valid core module: magic + version 1. Not a component.
    const CORE_MODULE: &[u8] = &[0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
    // The committed sample is a real component.
    const COMPONENT: &[u8] = include_bytes!("../../../../engine/tests/fixtures/sdk_sample.wasm");

    #[test]
    fn core_module_is_not_a_component() {
        assert_eq!(is_component(CORE_MODULE), Ok(false));
    }

    #[test]
    fn real_component_is_detected() {
        assert_eq!(is_component(COMPONENT), Ok(true));
    }

    #[test]
    fn garbage_is_rejected() {
        assert!(is_component(&[0, 1, 2, 3]).is_err());
    }

    #[test]
    fn component_validates() {
        assert!(validate(COMPONENT).is_ok());
    }
}
