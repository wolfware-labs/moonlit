use crate::plugin::artifact::{PluginArtifact, PluginArtifactMetadata};
use crate::plugin::publish::error::PublishError;
use crate::plugin::resolver::oci::auth::{is_auth_failure, resolve_auth};
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

#[must_use]
pub fn new_push_client() -> OciPushClient {
  OciPushClient(Client::default())
}

#[cfg(test)]
impl OciPushClient {
  #[must_use]
  fn with_plain_http_for(registry: &str) -> Self {
    Self(Client::new(oci_client::client::ClientConfig {
      protocol: oci_client::client::ClientProtocol::HttpsExcept(vec![registry.to_string()]),
      ..Default::default()
    }))
  }
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
      .map_err(|e| map_push_error(&e))
  }
}

#[must_use]
fn map_push_error(err: &oci_client::errors::OciDistributionError) -> PublishError {
  let msg = err.to_string();
  if is_auth_failure(err) {
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

#[must_use]
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
    let result = publish_plugin("reg/w/git:1", b"\0asm".to_vec(), metadata(), home.path(), &client).await;
    assert!(matches!(result, Err(PublishError::Auth(_))), "{result:?}");
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
    let result = publish_plugin("::not a ref::", b"x".to_vec(), metadata(), home.path(), &client).await;
    assert!(matches!(result, Err(PublishError::InvalidReference(_))), "{result:?}");
  }

  async fn push_registry() -> (wiremock::MockServer, String) {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    let server = MockServer::start().await;
    let host = server.address().to_string();
    Mock::given(method("GET"))
      .and(path("/v2/"))
      .respond_with(ResponseTemplate::new(200))
      .mount(&server)
      .await;
    (server, host)
  }

  #[tokio::test]
  async fn oci_push_client_uploads_blobs_and_the_manifest() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};
    let (server, host) = push_registry().await;
    let session = "/v2/w/git/blobs/uploads/session";
    Mock::given(method("POST"))
      .and(path("/v2/w/git/blobs/uploads/"))
      .respond_with(ResponseTemplate::new(202).insert_header("Location", session))
      .mount(&server)
      .await;
    Mock::given(method("PATCH"))
      .and(path(session))
      .respond_with(
        ResponseTemplate::new(202)
          .insert_header("Location", session)
          .insert_header("Range", "0-8"),
      )
      .mount(&server)
      .await;
    Mock::given(method("PUT"))
      .and(path(session))
      .respond_with(ResponseTemplate::new(201).insert_header("Location", "/v2/w/git/blobs/sha256:blob"))
      .mount(&server)
      .await;
    Mock::given(method("PUT"))
      .and(path("/v2/w/git/manifests/2.0.0"))
      .respond_with(ResponseTemplate::new(201).insert_header("Location", "/v2/w/git/manifests/sha256:feed"))
      .mount(&server)
      .await;
    let home = tempfile::tempdir().unwrap();

    let outcome = publish_plugin(
      &format!("{host}/w/git:2.0.0"),
      b"\0asm-body".to_vec(),
      metadata(),
      home.path(),
      &OciPushClient::with_plain_http_for(&host),
    )
    .await
    .unwrap();

    assert_eq!(outcome.reference, format!("oci://{host}/w/git:2.0.0"));
    assert_eq!(outcome.digest, "sha256:feed");
    assert_eq!(outcome.size, 9);
  }

  async fn push_failing_with(status: u16) -> Result<PublishOutcome, PublishError> {
    use wiremock::matchers::method;
    use wiremock::{Mock, ResponseTemplate};
    let (server, host) = push_registry().await;
    Mock::given(method("POST"))
      .respond_with(ResponseTemplate::new(status).set_body_string("nope"))
      .mount(&server)
      .await;
    let home = tempfile::tempdir().unwrap();
    publish_plugin(
      &format!("{host}/w/git:2.0.0"),
      b"\0asm".to_vec(),
      metadata(),
      home.path(),
      &OciPushClient::with_plain_http_for(&host),
    )
    .await
  }

  #[tokio::test]
  async fn oci_push_client_maps_a_401_to_an_auth_error() {
    let result = push_failing_with(401).await;
    assert!(matches!(result, Err(PublishError::Auth(_))), "{result:?}");
  }

  #[tokio::test]
  async fn oci_push_client_maps_server_errors_to_network_errors() {
    let result = push_failing_with(500).await;
    assert!(matches!(result, Err(PublishError::Network(_))), "{result:?}");
  }

  #[test]
  fn digest_is_unknown_when_the_manifest_url_has_none() {
    assert_eq!(digest_from_url("https://reg/v2/w/git/manifests/2.0.0"), None);
  }

  #[test]
  fn the_production_push_client_is_constructible() {
    let _client = new_push_client();
  }
}
