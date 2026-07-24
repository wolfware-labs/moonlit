//! `oci://` publishing (§8.1) — the push half of OCI, mirroring `resolve/oci.rs`. Holds the
//! [`PushClient`] seam (so `publish_plugin` is unit-tested against a mock) plus artifact assembly.

use std::collections::BTreeMap;
use std::path::Path;

use oci_client::client::{Config, ImageLayer, PushResponse};
use oci_client::manifest::OciImageManifest;
use oci_client::secrets::RegistryAuth;
use oci_client::{Client, Reference};

use crate::resolve::auth::resolve_auth;
use crate::resolve::oci::CONFIG_MEDIA_TYPE;

/// Manifest artifactType for Moonlit plugin artifacts (§8.1).
pub const ARTIFACT_TYPE: &str = "application/vnd.wasm.component.v1+wasm";
/// The single-layer media type (§8.1).
pub const LAYER_MEDIA_TYPE: &str = "application/wasm";
/// The fixed WIT world a Moonlit plugin implements (§8.1).
pub const PLUGIN_WORLD: &str = "moonlit:plugin@0.1.0";

/// Everything needed to describe a plugin artifact, gathered by the CLI
/// (component introspection + best-effort crate facts).
#[derive(Debug, Clone)]
pub struct PublishMeta {
    pub plugin_name: String,
    pub version: String,
    pub description: String,
    pub source: Option<String>,
    pub licenses: Option<String>,
    pub middlewares: Vec<String>,
    pub sdk_version: Option<String>,
}

/// The result of a successful push.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishOutcome {
    pub reference: String,
    pub digest: String,
    pub size: u64,
}

/// Publish failure classes (mirrors `ResolveError` classification).
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum PublishError {
    #[error("invalid plugin reference: {0}")]
    #[diagnostic(code(moonlit::publish::invalid_reference))]
    InvalidReference(String),
    #[error("authentication failed for registry: {0}")]
    #[diagnostic(code(moonlit::publish::auth))]
    Auth(String),
    #[error("registry error while publishing: {0}")]
    #[diagnostic(code(moonlit::publish::network))]
    Network(String),
    #[error("publish I/O error: {0}")]
    #[diagnostic(code(moonlit::publish::io))]
    Io(String),
}

/// A narrow seam over the OCI push, so `publish_plugin` is testable without a network.
#[allow(async_fn_in_trait)]
pub trait PushClient {
    async fn push(
        &self,
        reference: &Reference,
        layers: &[ImageLayer],
        config: Config,
        auth: &RegistryAuth,
        manifest: OciImageManifest,
    ) -> Result<PushResponse, PublishError>;
}

/// The real push client, backed by `oci-client` (rustls transport).
pub struct OciPushClient(Client);

/// Construct the real push client with default configuration.
pub fn new_push_client() -> OciPushClient {
    OciPushClient(Client::default())
}

impl PushClient for OciPushClient {
    async fn push(
        &self,
        reference: &Reference,
        layers: &[ImageLayer],
        config: Config,
        auth: &RegistryAuth,
        manifest: OciImageManifest,
    ) -> Result<PushResponse, PublishError> {
        self.0
            .push(reference, layers, config, auth, Some(manifest))
            .await
            .map_err(map_push_error)
    }
}

/// Classify an `oci-client` push error (auth vs everything-else).
fn map_push_error(err: oci_client::errors::OciDistributionError) -> PublishError {
    let msg = err.to_string();
    let lower = msg.to_lowercase();
    if lower.contains("unauthorized")
        || lower.contains("authentication")
        || lower.contains("401")
        || lower.contains("403")
    {
        PublishError::Auth(msg)
    } else {
        PublishError::Network(msg)
    }
}

/// Build the OCI config blob, wasm layer, and artifact manifest for a plugin (§8.1). Pure.
pub(crate) fn assemble_artifact(
    wasm: &[u8],
    meta: &PublishMeta,
) -> (Config, ImageLayer, OciImageManifest) {
    let layer = ImageLayer::new(wasm.to_vec(), LAYER_MEDIA_TYPE.to_string(), None);
    let layer_digest = layer.sha256_digest();

    // The `moonlit` config block (§8.1). `sdkVersion` is omitted entirely when absent.
    let mut moonlit = serde_json::Map::new();
    moonlit.insert("world".into(), PLUGIN_WORLD.into());
    moonlit.insert(
        "middlewares".into(),
        serde_json::Value::from(meta.middlewares.clone()),
    );
    if let Some(sdk) = &meta.sdk_version {
        moonlit.insert("sdkVersion".into(), sdk.clone().into());
    }
    let config_json = serde_json::json!({
        "layerDigests": [layer_digest],
        "moonlit": serde_json::Value::Object(moonlit),
    });
    let config_bytes = serde_json::to_vec(&config_json).expect("config json serializes");
    let config = Config::new(config_bytes, CONFIG_MEDIA_TYPE.to_string(), None);

    let annotations = build_annotations(meta);
    let mut manifest =
        OciImageManifest::build(std::slice::from_ref(&layer), &config, Some(annotations));
    manifest.artifact_type = Some(ARTIFACT_TYPE.to_string());
    (config, layer, manifest)
}

/// OCI image annotations (§8.1); optional fields omitted when empty/None.
fn build_annotations(meta: &PublishMeta) -> BTreeMap<String, String> {
    let mut a = BTreeMap::new();
    a.insert(
        "org.opencontainers.image.title".into(),
        meta.plugin_name.clone(),
    );
    a.insert(
        "org.opencontainers.image.version".into(),
        meta.version.clone(),
    );
    a.insert(
        "dev.moonlitbuild.plugin.name".into(),
        meta.plugin_name.clone(),
    );
    if !meta.description.is_empty() {
        a.insert(
            "org.opencontainers.image.description".into(),
            meta.description.clone(),
        );
    }
    if let Some(source) = &meta.source {
        a.insert("org.opencontainers.image.source".into(), source.clone());
    }
    if let Some(licenses) = &meta.licenses {
        a.insert("org.opencontainers.image.licenses".into(), licenses.clone());
    }
    a
}

/// Publish a plugin component to an `oci://` registry (§8). Resolves credentials from `home`
/// (Docker config, then Moonlit credentials), assembles the artifact, and pushes it.
pub async fn publish_plugin<C: PushClient>(
    raw_ref: &str,
    wasm: Vec<u8>,
    meta: PublishMeta,
    home: &Path,
    client: &C,
) -> Result<PublishOutcome, PublishError> {
    let reference: Reference = raw_ref
        .parse()
        .map_err(|e| PublishError::InvalidReference(format!("'{raw_ref}': {e}")))?;
    let auth = resolve_auth(reference.registry(), home);
    let size = wasm.len() as u64;
    let (config, layer, manifest) = assemble_artifact(&wasm, &meta);
    let response = client
        .push(&reference, &[layer], config, &auth, manifest)
        .await?;
    let digest = digest_from_url(&response.manifest_url).unwrap_or_else(|| "unknown".to_string());
    Ok(PublishOutcome {
        reference: format!("oci://{raw_ref}"),
        digest,
        size,
    })
}

/// Extract `sha256:<hex>` from a manifest URL (`…/manifests/sha256:abcd…`).
fn digest_from_url(url: &str) -> Option<String> {
    let idx = url.find("sha256:")?;
    Some(url[idx..].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta() -> PublishMeta {
        PublishMeta {
            plugin_name: "git".into(),
            version: "2.0.0".into(),
            description: "Git plugin".into(),
            source: Some("https://example.com/git".into()),
            licenses: Some("Elastic-2.0".into()),
            middlewares: vec!["build".into(), "test".into()],
            sdk_version: Some("0.1.0".into()),
        }
    }

    #[test]
    fn assemble_builds_config_layer_and_manifest() {
        let (config, layer, manifest) = assemble_artifact(b"\0asm-body", &meta());

        assert_eq!(config.media_type, CONFIG_MEDIA_TYPE);
        assert_eq!(layer.media_type, LAYER_MEDIA_TYPE);
        assert_eq!(layer.data.as_ref(), b"\0asm-body");
        assert_eq!(manifest.artifact_type.as_deref(), Some(ARTIFACT_TYPE));

        let cfg: serde_json::Value = serde_json::from_slice(&config.data).unwrap();
        assert_eq!(cfg["moonlit"]["world"], PLUGIN_WORLD);
        assert_eq!(
            cfg["moonlit"]["middlewares"],
            serde_json::json!(["build", "test"])
        );
        assert_eq!(cfg["moonlit"]["sdkVersion"], "0.1.0");
        assert!(
            cfg["layerDigests"][0]
                .as_str()
                .unwrap()
                .starts_with("sha256:")
        );

        let ann = manifest.annotations.unwrap();
        assert_eq!(ann["org.opencontainers.image.title"], "git");
        assert_eq!(ann["org.opencontainers.image.version"], "2.0.0");
        assert_eq!(ann["dev.moonlitbuild.plugin.name"], "git");
        assert_eq!(ann["org.opencontainers.image.description"], "Git plugin");
        assert_eq!(
            ann["org.opencontainers.image.source"],
            "https://example.com/git"
        );
        assert_eq!(ann["org.opencontainers.image.licenses"], "Elastic-2.0");
    }

    #[test]
    fn assemble_omits_sdk_version_and_optional_annotations_when_absent() {
        let m = PublishMeta {
            description: String::new(),
            source: None,
            licenses: None,
            sdk_version: None,
            ..meta()
        };
        let (config, _layer, manifest) = assemble_artifact(b"\0asm", &m);
        let cfg: serde_json::Value = serde_json::from_slice(&config.data).unwrap();
        assert!(cfg["moonlit"].get("sdkVersion").is_none());
        let ann = manifest.annotations.unwrap();
        assert!(!ann.contains_key("org.opencontainers.image.description"));
        assert!(!ann.contains_key("org.opencontainers.image.source"));
        assert!(!ann.contains_key("org.opencontainers.image.licenses"));
    }

    // A mock push client that records what it was handed.
    enum MockOutcome {
        Ok { manifest_url: String },
        Auth,
    }
    struct Seen {
        layer_media: String,
        layer_data: Vec<u8>,
        config_media: String,
        config_json: serde_json::Value,
        artifact_type: Option<String>,
    }
    struct MockPush {
        seen: std::sync::Mutex<Option<Seen>>,
        outcome: MockOutcome,
    }
    impl PushClient for MockPush {
        async fn push(
            &self,
            _reference: &Reference,
            layers: &[ImageLayer],
            config: Config,
            _auth: &RegistryAuth,
            manifest: OciImageManifest,
        ) -> Result<PushResponse, PublishError> {
            *self.seen.lock().unwrap() = Some(Seen {
                layer_media: layers[0].media_type.clone(),
                layer_data: layers[0].data.to_vec(),
                config_media: config.media_type.clone(),
                config_json: serde_json::from_slice(&config.data).unwrap(),
                artifact_type: manifest.artifact_type.clone(),
            });
            match &self.outcome {
                MockOutcome::Ok { manifest_url } => Ok(PushResponse {
                    config_url: "cfg".into(),
                    manifest_url: manifest_url.clone(),
                }),
                MockOutcome::Auth => Err(PublishError::Auth("401 Unauthorized".into())),
            }
        }
    }

    #[tokio::test]
    async fn publish_pushes_one_artifact_and_returns_digest() {
        let home = tempfile::tempdir().unwrap();
        let client = MockPush {
            seen: std::sync::Mutex::new(None),
            outcome: MockOutcome::Ok {
                manifest_url: "https://reg/v2/w/git/manifests/sha256:abc123".into(),
            },
        };
        let outcome = publish_plugin(
            "reg.example.com/w/git:2.0.0",
            b"\0asm-body".to_vec(),
            meta(),
            home.path(),
            &client,
        )
        .await
        .unwrap();

        assert_eq!(outcome.reference, "oci://reg.example.com/w/git:2.0.0");
        assert_eq!(outcome.digest, "sha256:abc123");
        assert_eq!(outcome.size, 9);

        let seen = client.seen.lock().unwrap().take().unwrap();
        assert_eq!(seen.layer_media, LAYER_MEDIA_TYPE);
        assert_eq!(seen.layer_data, b"\0asm-body");
        assert_eq!(seen.config_media, CONFIG_MEDIA_TYPE);
        assert_eq!(seen.artifact_type.as_deref(), Some(ARTIFACT_TYPE));
        assert_eq!(seen.config_json["moonlit"]["world"], PLUGIN_WORLD);
    }

    #[tokio::test]
    async fn publish_maps_auth_failure() {
        let home = tempfile::tempdir().unwrap();
        let client = MockPush {
            seen: std::sync::Mutex::new(None),
            outcome: MockOutcome::Auth,
        };
        let err = match publish_plugin(
            "reg/w/git:1",
            b"\0asm".to_vec(),
            meta(),
            home.path(),
            &client,
        )
        .await
        {
            Ok(_) => panic!("expected auth failure"),
            Err(e) => e,
        };
        assert!(matches!(err, PublishError::Auth(_)));
    }

    #[tokio::test]
    async fn publish_rejects_invalid_reference() {
        let home = tempfile::tempdir().unwrap();
        let client = MockPush {
            seen: std::sync::Mutex::new(None),
            outcome: MockOutcome::Ok {
                manifest_url: "sha256:x".into(),
            },
        };
        let err = match publish_plugin("::not a ref::", b"x".to_vec(), meta(), home.path(), &client)
            .await
        {
            Ok(_) => panic!("expected invalid reference"),
            Err(e) => e,
        };
        assert!(matches!(err, PublishError::InvalidReference(_)));
    }
}
