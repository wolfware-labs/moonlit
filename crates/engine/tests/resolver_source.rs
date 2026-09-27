use std::path::PathBuf;

use moonlit_engine::plugin::resolver::{PluginSource, ResolveError, ResolveOptions};

#[test]
fn oci_file_and_http_urls_parse_into_their_sources() {
  assert_eq!(
    "oci://ghcr.io/acme/git:1.0.0".parse::<PluginSource>().unwrap(),
    PluginSource::Oci("ghcr.io/acme/git:1.0.0".to_string())
  );
  assert_eq!(
    "file://plugins/git.wasm".parse::<PluginSource>().unwrap(),
    PluginSource::File(PathBuf::from("plugins/git.wasm"))
  );
  assert_eq!(
    "http://example.com/git.wasm".parse::<PluginSource>().unwrap(),
    PluginSource::Http("http://example.com/git.wasm".to_string())
  );
  assert_eq!(
    "  https://example.com/git.wasm  ".parse::<PluginSource>().unwrap(),
    PluginSource::Http("https://example.com/git.wasm".to_string())
  );
}

#[test]
fn an_unknown_scheme_is_unsupported() {
  let err = "ftp://example.com/git.wasm".parse::<PluginSource>().unwrap_err();
  let ResolveError::UnsupportedScheme { scheme, hint } = err else {
    panic!("expected UnsupportedScheme, got {err:?}");
  };
  assert_eq!(scheme, "ftp");
  assert!(hint.contains("oci, file, http, https"));
}

#[test]
fn a_url_without_a_scheme_is_an_invalid_reference() {
  let err = "ghcr.io/acme/git:1.0.0".parse::<PluginSource>().unwrap_err();
  let ResolveError::InvalidReference(msg) = err else {
    panic!("expected InvalidReference, got {err:?}");
  };
  assert!(msg.contains("has no URL scheme"), "{msg}");
}

#[test]
fn default_resolve_options_are_online_with_a_fifteen_minute_tag_ttl() {
  let opts = ResolveOptions::default();
  assert!(!opts.offline);
  assert_eq!(opts.tag_ttl, std::time::Duration::from_mins(15));
}

#[tokio::test]
async fn resolving_an_oci_reference_offline_without_a_cache_is_an_offline_miss() {
  let dir = tempfile::tempdir().unwrap();
  let cache =
    moonlit_engine::cache::Cache::with_root_and_clock(dir.path().to_path_buf(), Box::new(moonlit_engine::cache::SystemClock));
  let opts = ResolveOptions {
    offline: true,
    ..ResolveOptions::default()
  };

  let err = moonlit_engine::plugin::resolver::resolve(
    &PluginSource::Oci("127.0.0.1:9/w/plugin:1.0.0".to_string()),
    &opts,
    &cache,
    None,
  )
  .await
  .unwrap_err();

  assert!(matches!(err, ResolveError::OfflineMiss(_)), "{err:?}");
}
