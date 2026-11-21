use moonlit_engine::plugin::middleware::MiddlewareInfo;
use moonlit_engine::plugin::{PluginMetadata, world};
use wasmparser::{Encoding, Parser, Payload, Validator, WasmFeatures};

use crate::display::display_safe;
use crate::plugin_probe::introspect;

#[expect(
  clippy::never_loop,
  reason = "only the first payload, the version header, says what the binary is"
)]
pub fn is_component(bytes: &[u8]) -> Result<bool, String> {
  for payload in Parser::new(0).parse_all(bytes) {
    match payload.map_err(|e| format!("not a valid wasm binary: {e}"))? {
      Payload::Version { encoding, .. } => return Ok(encoding == Encoding::Component),
      _ => break,
    }
  }
  Err("empty or headerless wasm binary".to_string())
}

fn validate(bytes: &[u8]) -> Result<(), String> {
  let mut v = Validator::new_with_features(WasmFeatures::all());
  v.validate_all(bytes)
    .map(|_types| ())
    .map_err(|e| format!("invalid component: {e}"))
}

/// Checks that `bytes` hold a valid component built for this CLI's plugin contract and reads its metadata and
/// middlewares, printing why when it cannot; `Err` holds the exit code. `target` names the file in messages.
pub async fn load_component(target: &str, bytes: &[u8]) -> Result<(PluginMetadata, Vec<MiddlewareInfo>), i32> {
  match is_component(bytes) {
    Ok(true) => {}
    Ok(false) => {
      eprintln!(
        "error: '{}' is a core wasm module, not a WASI-P2 component (build with `moonlit plugin build`)",
        display_safe(target)
      );
      return Err(2);
    }
    Err(e) => {
      eprintln!("error: {e}");
      return Err(2);
    }
  }
  if let Err(e) = validate(bytes) {
    eprintln!("error: {e}");
    return Err(2);
  }
  if let Err(e) = world::check(bytes) {
    eprintln!("error: {}", display_safe(&format!("'{target}': {e}")));
    return Err(2);
  }
  introspect(bytes).await.map_err(|e| {
    eprintln!("error: failed to instantiate component: {e}");
    3
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  const CORE_MODULE: &[u8] = &[0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
  const COMPONENT: &[u8] = include_bytes!("../../../../engine/tests/fixtures/pdk_sample.wasm");

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
  fn empty_input_is_rejected() {
    assert!(is_component(&[]).is_err());
  }

  #[test]
  fn truncated_component_fails_validation() {
    assert!(validate(&COMPONENT[..COMPONENT.len() / 2]).is_err());
  }

  #[test]
  fn component_validates() {
    assert!(validate(COMPONENT).is_ok());
  }
}
