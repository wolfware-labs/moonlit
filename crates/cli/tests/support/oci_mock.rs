use sha2::{Digest, Sha256};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const MANIFEST_MEDIA_TYPE: &str = "application/vnd.oci.image.manifest.v1+json";
const CONFIG_MEDIA_TYPE: &str = "application/vnd.wasm.config.v0+json";

fn digest(bytes: &[u8]) -> String {
  format!("sha256:{}", hex::encode(Sha256::digest(bytes)))
}

/// Serves `wasm` as the OCI artifact `{repo}:{tag}` from `server`, with `config.moonlit.world = world`.
pub async fn serve_plugin(server: &MockServer, repo: &str, tag: &str, wasm: &[u8], world: &str) -> String {
  let config = serde_json::to_vec(&serde_json::json!({ "moonlit": { "world": world, "middlewares": [] } })).unwrap();
  let config_digest = digest(&config);
  let layer_digest = digest(wasm);
  let manifest = serde_json::to_vec(&serde_json::json!({
    "schemaVersion": 2,
    "mediaType": MANIFEST_MEDIA_TYPE,
    "config": { "mediaType": CONFIG_MEDIA_TYPE, "digest": config_digest, "size": config.len() },
    "layers": [{ "mediaType": "application/wasm", "digest": layer_digest, "size": wasm.len() }],
  }))
  .unwrap();
  let manifest_digest = digest(&manifest);
  let manifest_response = ResponseTemplate::new(200)
    .insert_header("Content-Type", MANIFEST_MEDIA_TYPE)
    .insert_header("Docker-Content-Digest", manifest_digest.as_str())
    .set_body_bytes(manifest);

  Mock::given(method("GET"))
    .and(path("/v2/"))
    .respond_with(ResponseTemplate::new(200))
    .mount(server)
    .await;
  Mock::given(method("GET"))
    .and(path(format!("/v2/{repo}/manifests/{tag}")))
    .respond_with(manifest_response.clone())
    .mount(server)
    .await;
  Mock::given(method("GET"))
    .and(path(format!("/v2/{repo}/manifests/{manifest_digest}")))
    .respond_with(manifest_response)
    .mount(server)
    .await;
  Mock::given(method("GET"))
    .and(path(format!("/v2/{repo}/blobs/{config_digest}")))
    .respond_with(ResponseTemplate::new(200).set_body_bytes(config))
    .mount(server)
    .await;
  Mock::given(method("GET"))
    .and(path(format!("/v2/{repo}/blobs/{layer_digest}")))
    .respond_with(ResponseTemplate::new(200).set_body_bytes(wasm.to_vec()))
    .mount(server)
    .await;
  manifest_digest
}
