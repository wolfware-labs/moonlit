use crate::plugin::PluginError;
use wasmparser::{Parser, Payload};

const INTERFACE_PREFIX: &str = "moonlit:plugin/";

/// Reads the plugin contract a component targets from its import names, as `moonlit:plugin@<version>`.
/// Returns `None` when the component imports nothing from the `moonlit:plugin` package.
pub fn component_world(bytes: &[u8]) -> Result<Option<String>, PluginError> {
  for payload in Parser::new(0).parse_all(bytes) {
    let payload = payload.map_err(|e| PluginError::Load(e.to_string()))?;
    if let Payload::ComponentImportSection(reader) = payload {
      for import in reader {
        let import = import.map_err(|e| PluginError::Load(e.to_string()))?;
        if let Some(version) = interface_version(import.name.name) {
          return Ok(Some(format!("moonlit:plugin@{version}")));
        }
      }
    }
  }
  Ok(None)
}

fn interface_version(import_name: &str) -> Option<&str> {
  let rest = import_name.strip_prefix(INTERFACE_PREFIX)?;
  let (_, version) = rest.split_once('@')?;
  Some(version)
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::path::Path;

  fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)).unwrap()
  }

  #[test]
  fn reads_world_of_current_fixture() {
    assert_eq!(
      component_world(&fixture("test_plugin.wasm")).unwrap().as_deref(),
      Some("moonlit:plugin@0.4.0")
    );
  }

  #[test]
  fn reads_world_of_old_fixture() {
    assert_eq!(
      component_world(&fixture("test_plugin_v0_3.wasm")).unwrap().as_deref(),
      Some("moonlit:plugin@0.3.0")
    );
  }

  #[test]
  fn component_without_moonlit_imports_has_no_world() {
    let bytes = wat::parse_str("(component)").unwrap();
    assert_eq!(component_world(&bytes).unwrap(), None);
  }

  #[test]
  fn garbage_bytes_are_a_load_error() {
    assert!(matches!(component_world(b"not wasm"), Err(PluginError::Load(_))));
  }
}
