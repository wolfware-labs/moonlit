use crate::plugin::resolver::ResolveError;
use crate::plugin::resolver::oci::auth::is_auth_failure;
use oci_client::errors::{OciDistributionError, OciErrorCode};
use oci_client::manifest::{OciDescriptor, OciImageManifest};
use oci_client::secrets::RegistryAuth;
use oci_client::{Client, Reference};

pub struct OciClient(Client);

impl OciClient {
  #[must_use]
  pub fn new() -> Self {
    Self(Client::default())
  }

  #[cfg(test)]
  #[must_use]
  pub fn with_plain_http_for(registry: &str) -> Self {
    Self(Client::new(oci_client::client::ClientConfig {
      protocol: oci_client::client::ClientProtocol::HttpsExcept(vec![registry.to_string()]),
      ..Default::default()
    }))
  }
}

impl OciClient {
  pub async fn pull_image_manifest(
    &self,
    reference: &Reference,
    auth: &RegistryAuth,
  ) -> Result<(OciImageManifest, String), ResolveError> {
    self
      .0
      .pull_image_manifest(reference, auth)
      .await
      .map_err(|e| OciClient::map_oci_error(&e))
  }

  pub async fn pull_blob(&self, reference: &Reference, descriptor: &OciDescriptor) -> Result<Vec<u8>, ResolveError> {
    let mut buf: Vec<u8> = Vec::with_capacity(usize::try_from(descriptor.size).unwrap_or(0));
    self
      .0
      .pull_blob(reference, descriptor, &mut buf)
      .await
      .map_err(|e| OciClient::map_oci_error(&e))?;
    Ok(buf)
  }

  #[must_use]
  fn map_oci_error(err: &OciDistributionError) -> ResolveError {
    let msg = err.to_string();
    if is_auth_failure(err) {
      return ResolveError::Auth(msg);
    }
    match err {
      OciDistributionError::ServerError { code: 404, .. } | OciDistributionError::ImageManifestNotFoundError(_) => {
        ResolveError::NotFound(msg)
      }
      OciDistributionError::RegistryError { envelope, .. }
        if envelope.errors.iter().any(|e| {
          matches!(
            e.code,
            OciErrorCode::ManifestUnknown | OciErrorCode::BlobUnknown | OciErrorCode::NameUnknown | OciErrorCode::NotFound
          )
        }) =>
      {
        ResolveError::NotFound(msg)
      }
      OciDistributionError::DigestError(_) => ResolveError::DigestMismatch(msg),
      _ => ResolveError::Network(msg),
    }
  }
}
