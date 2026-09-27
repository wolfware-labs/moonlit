use crate::plugin::artifact::PluginArtifact;
use crate::plugin::publish::error::PublishError;
use crate::plugin::resolver::oci::auth::resolve_auth;
use oci_client::client::PushResponse;
use oci_client::secrets::RegistryAuth;
use oci_client::{Client, Reference};
use std::path::Path;
use wasmtime_wasi::async_trait;

#[async_trait]
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
      .push(reference, layers, config, auth, Some(manifest))
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
  meta: PublishMeta,
  home: &Path,
  client: &C,
) -> Result<PublishOutcome, PublishError> {
  let reference: Reference = raw_ref
    .parse()
    .map_err(|e| PublishError::InvalidReference(format!("'{raw_ref}': {e}")))?;
  let auth = resolve_auth(reference.registry(), home);
  let size = wasm.len() as u64;
  let artifact = PluginArtifact::assemble(&wasm, &meta);
  let response = client.push(&reference, &[layer], config, &auth, manifest).await?;
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
