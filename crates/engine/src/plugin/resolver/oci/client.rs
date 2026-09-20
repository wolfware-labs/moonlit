use crate::plugin::resolver::ResolveError;
use oci_client::manifest::{OciDescriptor, OciImageManifest};
use oci_client::secrets::RegistryAuth;
use oci_client::{Client, Reference};

pub struct OciClient(Client);

impl OciClient {
    pub fn new() -> Self {
        Self(Client::default())
    }
}

impl OciClient {
    pub async fn pull_image_manifest(
        &self,
        reference: &Reference,
        auth: &RegistryAuth,
    ) -> Result<(OciImageManifest, String), ResolveError> {
        self.0
            .pull_image_manifest(reference, auth)
            .await
            .map_err(OciClient::map_oci_error)
    }

    pub async fn pull_blob(
        &self,
        reference: &Reference,
        descriptor: &OciDescriptor,
    ) -> Result<Vec<u8>, ResolveError> {
        let mut buf: Vec<u8> = Vec::with_capacity(descriptor.size.max(0) as usize);
        self.0
            .pull_blob(reference, descriptor, &mut buf)
            .await
            .map_err(OciClient::map_oci_error)?;
        Ok(buf)
    }

    fn map_oci_error(err: oci_client::errors::OciDistributionError) -> ResolveError {
        let msg = err.to_string();
        let lower = msg.to_lowercase();
        if lower.contains("unauthorized")
            || lower.contains("authentication")
            || lower.contains("401")
            || lower.contains("403")
        {
            ResolveError::Auth(msg)
        } else if lower.contains("not found") || lower.contains("404") {
            ResolveError::NotFound(msg)
        } else if lower.contains("digest") && lower.contains("mismatch") {
            ResolveError::DigestMismatch(msg)
        } else {
            ResolveError::Network(msg)
        }
    }
}
