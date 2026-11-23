use crate::plugin::resolver::oci::auth::is_auth_failure;
use crate::plugin::resolver::{CONNECT_TIMEOUT, READ_TIMEOUT, ResolveError};
use oci_client::client::{ClientConfig, ClientProtocol};
use oci_client::errors::{OciDistributionError, OciErrorCode};
use oci_client::manifest::{OciDescriptor, OciImageManifest};
use oci_client::secrets::RegistryAuth;
use oci_client::{Client, Reference};
use std::time::Duration;

pub struct OciClient(Client);

impl OciClient {
  #[must_use]
  pub fn for_registry(registry: &str) -> Self {
    if crate::registry::is_loopback(registry) {
      Self::with_plain_http_for(registry)
    } else {
      Self::with_config(ClientProtocol::default(), CONNECT_TIMEOUT, READ_TIMEOUT)
    }
  }

  #[must_use]
  pub fn with_plain_http_for(registry: &str) -> Self {
    Self::with_config(
      ClientProtocol::HttpsExcept(vec![registry.to_string()]),
      CONNECT_TIMEOUT,
      READ_TIMEOUT,
    )
  }

  #[must_use]
  #[cfg(test)]
  pub fn with_plain_http_and_timeouts(registry: &str, connect: Duration, read: Duration) -> Self {
    Self::with_config(ClientProtocol::HttpsExcept(vec![registry.to_string()]), connect, read)
  }

  fn with_config(protocol: ClientProtocol, connect: Duration, read: Duration) -> Self {
    Self(
      Client::try_from(client_config(protocol, connect, read))
        .expect("the OCI client builds with static timeouts and the default TLS backend"),
    )
  }
}

#[must_use]
pub(crate) fn client_config(protocol: ClientProtocol, connect: Duration, read: Duration) -> ClientConfig {
  ClientConfig {
    protocol,
    connect_timeout: Some(connect),
    read_timeout: Some(read),
    ..Default::default()
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
      OciDistributionError::RequestError(e) if e.is_connect() || e.is_timeout() || e.is_request() => ResolveError::Network(msg),
      _ => ResolveError::Registry(msg),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn client_config_sets_both_timeouts() {
    let config = client_config(ClientProtocol::default(), CONNECT_TIMEOUT, READ_TIMEOUT);
    assert_eq!(config.connect_timeout, Some(CONNECT_TIMEOUT));
    assert_eq!(config.read_timeout, Some(READ_TIMEOUT));
  }

  #[tokio::test]
  async fn a_request_that_cannot_be_built_is_a_registry_error() {
    let err = reqwest::Client::new().get("not a url").send().await.unwrap_err();
    assert!(!err.is_connect() && !err.is_timeout() && !err.is_request());
    let mapped = OciClient::map_oci_error(&OciDistributionError::RequestError(err));
    assert!(matches!(mapped, ResolveError::Registry(_)), "got: {mapped:?}");
  }

  #[tokio::test]
  async fn a_body_that_times_out_is_a_network_error() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
      let (mut stream, _) = listener.accept().unwrap();
      let mut buf = [0u8; 1024];
      let _ = stream.read(&mut buf);
      stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\n\r\n").unwrap();
      std::thread::sleep(Duration::from_secs(2));
    });
    let client = reqwest::Client::builder()
      .timeout(Duration::from_millis(300))
      .build()
      .unwrap();
    let response = client.get(format!("http://{addr}/")).send().await.unwrap();
    let err = response.bytes().await.unwrap_err();
    assert!(err.is_timeout() && !err.is_connect() && !err.is_request());
    let mapped = OciClient::map_oci_error(&OciDistributionError::RequestError(err));
    assert!(matches!(mapped, ResolveError::Network(_)), "got: {mapped:?}");
    server.join().unwrap();
  }
}
