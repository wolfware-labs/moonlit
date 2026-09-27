use crate::plugin::resolver::oci::CONFIG_MEDIA_TYPE;
use oci_client::client::{Config, ImageLayer};
use oci_client::manifest::OciImageManifest;
use std::collections::BTreeMap;

const ARTIFACT_TYPE: &str = "application/vnd.wasm.component.v1+wasm";
pub const LAYER_MEDIA_TYPE: &str = "application/wasm";
pub const PLUGIN_WORLD: &str = "moonlit:plugin@0.3.0";

pub struct PluginArtifact {
  config: Config,
  layer: ImageLayer,
  manifest: OciImageManifest,
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
