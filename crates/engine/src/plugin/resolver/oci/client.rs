use crate::plugin::resolver::ResolveError;
use crate::plugin::resolver::oci::auth::is_auth_failure;
use oci_client::errors::{OciDistributionError, OciErrorCode};
use oci_client::manifest::{OciDescriptor, OciImageManifest};
use oci_client::secrets::RegistryAuth;
use oci_client::{Client, Reference};

pub struct OciClient(Client);

impl OciClient {
  #[must_use]
  pub fn for_registry(registry: &str) -> Self {
    if is_loopback(registry) {
      Self::with_plain_http_for(registry)
    } else {
      Self(Client::default())
    }
  }

  #[must_use]
  pub fn with_plain_http_for(registry: &str) -> Self {
    Self(Client::new(oci_client::client::ClientConfig {
      protocol: oci_client::client::ClientProtocol::HttpsExcept(vec![registry.to_string()]),
      ..Default::default()
    }))
  }
}

#[must_use]
fn is_loopback(registry: &str) -> bool {
  let host = match registry.strip_prefix('[') {
    Some(rest) => rest.split_once(']').map_or(rest, |(h, _)| h),
    None => registry.split_once(':').map_or(registry, |(h, _)| h),
  };
  host.eq_ignore_ascii_case("localhost") || host.parse::<std::net::IpAddr>().is_ok_and(|ip| ip.is_loopback())
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

#[cfg(test)]
mod tests {
  use super::is_loopback;

  #[test]
  fn loopback_registries_are_recognised() {
    for registry in ["localhost:5000", "127.0.0.1:8080", "127.0.0.1", "[::1]:5000", "LOCALHOST"] {
      assert!(is_loopback(registry), "{registry}");
    }
  }

  #[test]
  fn remote_registries_are_not_loopback() {
    for registry in [
      "registry.moonlit.rs",
      "ghcr.io:443",
      "10.0.0.1:5000",
      "[2001:db8::1]:5000",
      "localhost.example.com",
      "127.evil.com:5000",
      "127.0.0.1.nip.io",
      "[::ffff:127.0.0.1]:5000",
    ] {
      assert!(!is_loopback(registry), "{registry}");
    }
  }
}
