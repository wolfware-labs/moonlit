use crate::plugin::resolver::oci::CONFIG_MEDIA_TYPE;
use oci_client::client::{Config, ImageLayer};
use oci_client::manifest::OciImageManifest;
use std::collections::BTreeMap;

const ARTIFACT_TYPE: &str = "application/vnd.wasm.component.v1+wasm";
pub const LAYER_MEDIA_TYPE: &str = "application/wasm";
pub const PLUGIN_WORLD: &str = "moonlit:plugin@0.3.0";

pub struct PluginArtifact {
  pub(crate) config: Config,
  pub(crate) layer: ImageLayer,
  pub(crate) manifest: OciImageManifest,
}

#[derive(Debug, Clone)]
pub struct PluginArtifactMetadata {
  pub plugin_name: String,
  pub version: String,
  pub description: String,
  pub source: Option<String>,
  pub licenses: Option<String>,
  pub middlewares: Vec<String>,
  pub sdk_version: Option<String>,
}

impl PluginArtifact {
  pub fn assemble(wasm: &[u8], meta: &PluginArtifactMetadata) -> PluginArtifact {
    let layer = ImageLayer::new(wasm.to_vec(), LAYER_MEDIA_TYPE.to_string(), None);
    let layer_digest = layer.sha256_digest();

    let mut moonlit = serde_json::Map::new();
    moonlit.insert("world".into(), PLUGIN_WORLD.into());
    moonlit.insert("middlewares".into(), serde_json::Value::from(meta.middlewares.clone()));
    if let Some(sdk) = &meta.sdk_version {
      moonlit.insert("sdkVersion".into(), sdk.clone().into());
    }
    let config_json = serde_json::json!({
        "layerDigests": [layer_digest],
        "moonlit": serde_json::Value::Object(moonlit),
    });
    let config_bytes = serde_json::to_vec(&config_json).expect("config json serializes");
    let config = Config::new(config_bytes, CONFIG_MEDIA_TYPE.to_string(), None);

    let annotations = Self::build_annotations(meta);
    let mut manifest = OciImageManifest::build(std::slice::from_ref(&layer), &config, Some(annotations));
    manifest.artifact_type = Some(ARTIFACT_TYPE.to_string());
    Self { config, layer, manifest }
  }

  fn build_annotations(meta: &PluginArtifactMetadata) -> BTreeMap<String, String> {
    let mut a = BTreeMap::new();
    a.insert("org.opencontainers.image.title".into(), meta.plugin_name.clone());
    a.insert("org.opencontainers.image.version".into(), meta.version.clone());
    a.insert("dev.moonlitbuild.plugin.name".into(), meta.plugin_name.clone());
    if !meta.description.is_empty() {
      a.insert("org.opencontainers.image.description".into(), meta.description.clone());
    }
    if let Some(source) = &meta.source {
      a.insert("org.opencontainers.image.source".into(), source.clone());
    }
    if let Some(licenses) = &meta.licenses {
      a.insert("org.opencontainers.image.licenses".into(), licenses.clone());
    }
    a
  }
}

#[cfg(test)]
pub(crate) mod tests {
  use super::*;

  pub(crate) fn metadata() -> PluginArtifactMetadata {
    PluginArtifactMetadata {
      plugin_name: "git".into(),
      version: "2.0.0".into(),
      description: "Git plugin".into(),
      source: Some("https://example.com/git".into()),
      licenses: Some("MIT OR Apache-2.0".into()),
      middlewares: vec!["build".into(), "test".into()],
      sdk_version: Some("0.1.0".into()),
    }
  }

  #[test]
  fn assemble_builds_config_layer_and_manifest() {
    let PluginArtifact { config, layer, manifest } = PluginArtifact::assemble(b"\0asm-body", &metadata());

    assert_eq!(config.media_type, CONFIG_MEDIA_TYPE);
    assert_eq!(layer.media_type, LAYER_MEDIA_TYPE);
    assert_eq!(layer.data.as_ref(), b"\0asm-body");
    assert_eq!(manifest.artifact_type.as_deref(), Some(ARTIFACT_TYPE));

    let cfg: serde_json::Value = serde_json::from_slice(&config.data).unwrap();
    assert_eq!(cfg["moonlit"]["world"], PLUGIN_WORLD);
    assert_eq!(cfg["moonlit"]["middlewares"], serde_json::json!(["build", "test"]));
    assert_eq!(cfg["moonlit"]["sdkVersion"], "0.1.0");
    assert!(cfg["layerDigests"][0].as_str().unwrap().starts_with("sha256:"));

    let ann = manifest.annotations.unwrap();
    assert_eq!(ann["org.opencontainers.image.title"], "git");
    assert_eq!(ann["org.opencontainers.image.version"], "2.0.0");
    assert_eq!(ann["dev.moonlitbuild.plugin.name"], "git");
    assert_eq!(ann["org.opencontainers.image.description"], "Git plugin");
    assert_eq!(ann["org.opencontainers.image.source"], "https://example.com/git");
    assert_eq!(ann["org.opencontainers.image.licenses"], "MIT OR Apache-2.0");
  }

  #[test]
  fn assemble_omits_sdk_version_and_optional_annotations_when_absent() {
    let m = PluginArtifactMetadata {
      description: String::new(),
      source: None,
      licenses: None,
      sdk_version: None,
      ..metadata()
    };
    let PluginArtifact { config, manifest, .. } = PluginArtifact::assemble(b"\0asm", &m);
    let cfg: serde_json::Value = serde_json::from_slice(&config.data).unwrap();
    assert!(cfg["moonlit"].get("sdkVersion").is_none());
    let ann = manifest.annotations.unwrap();
    assert!(!ann.contains_key("org.opencontainers.image.description"));
    assert!(!ann.contains_key("org.opencontainers.image.source"));
    assert!(!ann.contains_key("org.opencontainers.image.licenses"));
  }
}
