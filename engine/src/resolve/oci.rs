//! `oci://` resolution (§8). This file holds the [`RegistryClient`] seam over `oci-client` (so the
//! orchestration in `resolve_oci` is unit-tested against a mock) plus artifact verification helpers.
#![allow(dead_code)] // TODO(Task 8): remove once the public resolve() dispatcher wires the OCI resolver.

use oci_client::manifest::{OciDescriptor, OciImageManifest};
use oci_client::secrets::RegistryAuth;
use oci_client::{Client, Reference};

use crate::resolve::ResolveError;

/// The OCI config media type Moonlit plugin artifacts use (§8.1).
pub(crate) const CONFIG_MEDIA_TYPE: &str = "application/vnd.wasm.config.v0+json";
/// Accepted layer media types (§8.1); the CNCF wasm-OCI convention plus the older deislabs constant.
pub(crate) const LAYER_MEDIA_TYPES: [&str; 2] = [
    "application/wasm",
    "application/vnd.wasm.content.layer.v1+wasm",
];

/// A narrow seam over the OCI registry, so `resolve_oci` can be tested without a network.
pub(crate) trait RegistryClient {
    /// Pull the (single-platform) image manifest and its digest.
    async fn pull_image_manifest(
        &self,
        reference: &Reference,
        auth: &RegistryAuth,
    ) -> Result<(OciImageManifest, String), ResolveError>;

    /// Pull a blob (config or layer) identified by its descriptor. The implementation verifies the
    /// content digest.
    async fn pull_blob(
        &self,
        reference: &Reference,
        descriptor: &OciDescriptor,
    ) -> Result<Vec<u8>, ResolveError>;
}

/// The real registry client, backed by `oci-client`.
pub(crate) struct OciClient(Client);

/// Construct the real client with default configuration (rustls transport).
pub(crate) fn new_client() -> OciClient {
    OciClient(Client::default())
}

impl RegistryClient for OciClient {
    async fn pull_image_manifest(
        &self,
        reference: &Reference,
        auth: &RegistryAuth,
    ) -> Result<(OciImageManifest, String), ResolveError> {
        self.0
            .pull_image_manifest(reference, auth)
            .await
            .map_err(map_oci_error)
    }

    async fn pull_blob(
        &self,
        reference: &Reference,
        descriptor: &OciDescriptor,
    ) -> Result<Vec<u8>, ResolveError> {
        let mut buf: Vec<u8> = Vec::with_capacity(descriptor.size.max(0) as usize);
        self.0
            .pull_blob(reference, descriptor, &mut buf)
            .await
            .map_err(map_oci_error)?;
        Ok(buf)
    }
}

/// Map an `oci-client` error to a [`ResolveError`], classifying auth/not-found/digest cases.
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

/// Verify the artifact is a Moonlit/wasm plugin by its config + layer media types (§8.1).
pub(crate) fn verify_media_types(
    config_media_type: &str,
    layer_media_type: &str,
) -> Result<(), ResolveError> {
    if config_media_type != CONFIG_MEDIA_TYPE {
        return Err(ResolveError::MediaTypeMismatch(format!(
            "config media type is '{config_media_type}', expected '{CONFIG_MEDIA_TYPE}'"
        )));
    }
    if !LAYER_MEDIA_TYPES.contains(&layer_media_type) {
        return Err(ResolveError::MediaTypeMismatch(format!(
            "layer media type is '{layer_media_type}', expected one of {LAYER_MEDIA_TYPES:?}"
        )));
    }
    Ok(())
}

/// Extract `moonlit.middlewares` from the OCI config JSON (§8.1), if present.
pub(crate) fn parse_middlewares(config_json: &[u8]) -> Option<Vec<String>> {
    let value: serde_json::Value = serde_json::from_slice(config_json).ok()?;
    let arr = value.get("moonlit")?.get("middlewares")?.as_array()?;
    Some(
        arr.iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_documented_wasm_media_types() {
        assert!(
            verify_media_types("application/vnd.wasm.config.v0+json", "application/wasm").is_ok()
        );
        assert!(
            verify_media_types(
                "application/vnd.wasm.config.v0+json",
                "application/vnd.wasm.content.layer.v1+wasm"
            )
            .is_ok()
        );
    }

    #[test]
    fn rejects_wrong_config_media_type() {
        let err = verify_media_types(
            "application/vnd.oci.image.config.v1+json",
            "application/wasm",
        )
        .unwrap_err();
        assert!(matches!(err, ResolveError::MediaTypeMismatch(_)));
    }

    #[test]
    fn rejects_wrong_layer_media_type() {
        let err = verify_media_types(
            "application/vnd.wasm.config.v0+json",
            "application/octet-stream",
        )
        .unwrap_err();
        assert!(matches!(err, ResolveError::MediaTypeMismatch(_)));
    }

    #[test]
    fn parses_middlewares_from_moonlit_block() {
        let json =
            br#"{"moonlit":{"world":"moonlit:plugin@2.0.0","middlewares":["build","test"]}}"#;
        assert_eq!(
            parse_middlewares(json),
            Some(vec!["build".to_string(), "test".to_string()])
        );
    }

    #[test]
    fn missing_moonlit_block_yields_none() {
        assert_eq!(parse_middlewares(br#"{"layerDigests":[]}"#), None);
    }

    #[test]
    fn malformed_config_json_yields_none() {
        assert_eq!(parse_middlewares(b"not json"), None);
    }
}
