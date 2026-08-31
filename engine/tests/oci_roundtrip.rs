//! Gated real publish→pull round-trip against a local OCI registry.
//!
//! Ignored by default so `cargo test` stays hermetic. To run it, start a registry
//! (`docker run -d -p 5000:5000 registry:2`) and:
//!   MOONLIT_TEST_OCI_REGISTRY=localhost:5000 \
//!     cargo test -p moonlit-engine --test oci_roundtrip -- --ignored

use moonlit_engine::cache::{Cache, SystemClock};
use moonlit_engine::publish::{PublishMeta, new_push_client, publish_plugin};
use moonlit_engine::resolve::{PluginSource, ResolveOptions, resolve};

#[tokio::test]
#[ignore = "requires a local OCI registry (set MOONLIT_TEST_OCI_REGISTRY)"]
async fn publish_then_pull_roundtrips() {
    let Ok(host) = std::env::var("MOONLIT_TEST_OCI_REGISTRY") else {
        eprintln!("skipping: MOONLIT_TEST_OCI_REGISTRY not set");
        return;
    };

    let wasm = include_bytes!("fixtures/pdk_sample.wasm").to_vec();
    let raw_ref = format!("{host}/moonlit-test/sample:0.0.1");
    let meta = PublishMeta {
        plugin_name: "sample".into(),
        version: "0.0.1".into(),
        description: "roundtrip fixture".into(),
        source: None,
        licenses: None,
        middlewares: vec![],
        sdk_version: Some("0.1.0".into()),
    };

    let home = tempfile::tempdir().unwrap();
    let outcome = publish_plugin(
        &raw_ref,
        wasm.clone(),
        meta,
        home.path(),
        &new_push_client(),
    )
    .await
    .expect("publish should succeed against the local registry");
    assert!(
        outcome.digest.starts_with("sha256:"),
        "digest = {}",
        outcome.digest
    );
    assert_eq!(outcome.size, wasm.len() as u64);

    // Pull it back into a fresh, isolated cache.
    let cache_dir = tempfile::tempdir().unwrap();
    let cache = Cache::with_root_and_clock(cache_dir.path().to_path_buf(), Box::new(SystemClock));
    let source = PluginSource::parse(&format!("oci://{raw_ref}")).unwrap();
    let resolved = resolve(&source, &ResolveOptions::default(), &cache, None)
        .await
        .expect("pull should succeed");
    let pulled = std::fs::read(&resolved.wasm_path).unwrap();
    assert_eq!(pulled, wasm, "pulled bytes must equal published bytes");
}
