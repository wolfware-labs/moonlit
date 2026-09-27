use crate::plugin::artifact::{PluginArtifact, PluginArtifactMetadata};
use crate::plugin::publish::error::PublishError;
use crate::plugin::resolver::oci::auth::resolve_auth;
use oci_client::client::PushResponse;
use oci_client::secrets::RegistryAuth;
use oci_client::{Client, Reference};
use std::path::Path;

#[allow(async_fn_in_trait)]
pub trait PushClient {
  async fn push(
    &self,
    reference: &Reference,
    auth: &RegistryAuth,
    artifact: &PluginArtifact,
  ) -> Result<PushResponse, PublishError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishOutcome {
  pub reference: String,
  pub digest: String,
  pub size: u64,
}

pub struct OciPushClient(Client);

pub fn new_push_client() -> OciPushClient {
  OciPushClient(Client::default())
}

impl PushClient for OciPushClient {
  async fn push(
    &self,
    reference: &Reference,
    auth: &RegistryAuth,
    artifact: &PluginArtifact,
  ) -> Result<PushResponse, PublishError> {
    self
      .0
      .push(
        reference,
        std::slice::from_ref(&artifact.layer),
        artifact.config.clone(),
        auth,
        Some(artifact.manifest.clone()),
      )
      .await
      .map_err(map_push_error)
  }
}

fn map_push_error(err: oci_client::errors::OciDistributionError) -> PublishError {
  let msg = err.to_string();
  let lower = msg.to_lowercase();
  if lower.contains("unauthorized") || lower.contains("authentication") || lower.contains("401") || lower.contains("403") {
    PublishError::Auth(msg)
  } else {
    PublishError::Network(msg)
  }
}

pub async fn publish_plugin<C: PushClient>(
  raw_ref: &str,
  wasm: Vec<u8>,
  meta: PluginArtifactMetadata,
  home: &Path,
  client: &C,
) -> Result<PublishOutcome, PublishError> {
  let reference: Reference = raw_ref
    .parse()
    .map_err(|e| PublishError::InvalidReference(format!("'{raw_ref}': {e}")))?;
  let auth = resolve_auth(reference.registry(), home);
  let size = wasm.len() as u64;
  let artifact = PluginArtifact::assemble(&wasm, &meta);
  let response = client.push(&reference, &auth, &artifact).await?;
  let digest = digest_from_url(&response.manifest_url).unwrap_or_else(|| "unknown".to_string());
  Ok(PublishOutcome {
    reference: format!("oci://{raw_ref}"),
    digest,
    size,
  })
}

fn digest_from_url(url: &str) -> Option<String> {
  let idx = url.find("sha256:")?;
  Some(url[idx..].to_string())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::plugin::artifact::tests::metadata;
  use crate::plugin::artifact::{LAYER_MEDIA_TYPE, PLUGIN_WORLD};
  use crate::plugin::resolver::oci::CONFIG_MEDIA_TYPE;

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
      _auth: &RegistryAuth,
      artifact: &PluginArtifact,
    ) -> Result<PushResponse, PublishError> {
      *self.seen.lock().unwrap() = Some(Seen {
        layer_media: artifact.layer.media_type.clone(),
        layer_data: artifact.layer.data.to_vec(),
        config_media: artifact.config.media_type.clone(),
        config_json: serde_json::from_slice(&artifact.config.data).unwrap(),
        artifact_type: artifact.manifest.artifact_type.clone(),
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
      metadata(),
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
    assert_eq!(seen.artifact_type.as_deref(), Some("application/vnd.wasm.component.v1+wasm"));
    assert_eq!(seen.config_json["moonlit"]["world"], PLUGIN_WORLD);
  }

  #[tokio::test]
  async fn publish_maps_auth_failure() {
    let home = tempfile::tempdir().unwrap();
    let client = MockPush {
      seen: std::sync::Mutex::new(None),
      outcome: MockOutcome::Auth,
    };
    let err = match publish_plugin("reg/w/git:1", b"\0asm".to_vec(), metadata(), home.path(), &client).await {
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
    let err = match publish_plugin("::not a ref::", b"x".to_vec(), metadata(), home.path(), &client).await {
      Ok(_) => panic!("expected invalid reference"),
      Err(e) => e,
    };
    assert!(matches!(err, PublishError::InvalidReference(_)));
  }
}
