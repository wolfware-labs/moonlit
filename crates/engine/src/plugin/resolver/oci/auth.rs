use std::collections::HashMap;
use std::path::Path;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use oci_client::errors::{OciDistributionError, OciErrorCode};
use oci_client::secrets::RegistryAuth;
use serde::Deserialize;

#[derive(Deserialize)]
struct DockerConfig {
  #[serde(default)]
  auths: HashMap<String, DockerAuthEntry>,
}

#[derive(Deserialize)]
struct DockerAuthEntry {
  #[serde(default)]
  auth: Option<String>,
}

#[derive(Deserialize)]
struct MoonlitCredentials {
  #[serde(default)]
  registries: HashMap<String, MoonlitRegistryCred>,
}

#[derive(Deserialize)]
struct MoonlitRegistryCred {
  token: Option<String>,
  username: Option<String>,
  password: Option<String>,
}

#[must_use]
pub fn resolve_auth(host: &str, home: &Path) -> RegistryAuth {
  if let Some(auth) = docker_auth(host, home) {
    return auth;
  }
  if let Some(auth) = moonlit_auth(host, home) {
    return auth;
  }
  RegistryAuth::Anonymous
}

#[must_use]
fn docker_auth(host: &str, home: &Path) -> Option<RegistryAuth> {
  let bytes = std::fs::read(home.join(".docker/config.json")).ok()?;
  let config: DockerConfig = serde_json::from_slice(&bytes).ok()?;
  let entry = config.auths.get(host)?;
  let encoded = entry.auth.as_deref()?;
  let decoded = STANDARD.decode(encoded).ok()?;
  let text = String::from_utf8(decoded).ok()?;
  let (user, pass) = text.split_once(':')?;
  Some(RegistryAuth::Basic(user.to_string(), pass.to_string()))
}

#[must_use]
fn moonlit_auth(host: &str, home: &Path) -> Option<RegistryAuth> {
  let text = std::fs::read_to_string(home.join(".config/moonlit/credentials.toml")).ok()?;
  let creds: MoonlitCredentials = toml::from_str(&text).ok()?;
  let cred = creds.registries.get(host)?;
  if let Some(token) = &cred.token {
    return Some(RegistryAuth::Bearer(token.clone()));
  }
  if let (Some(u), Some(p)) = (&cred.username, &cred.password) {
    return Some(RegistryAuth::Basic(u.clone(), p.clone()));
  }
  None
}

#[must_use]
pub(crate) fn is_auth_failure(err: &OciDistributionError) -> bool {
  match err {
    OciDistributionError::UnauthorizedError { .. }
    | OciDistributionError::AuthenticationFailure(_)
    | OciDistributionError::ServerError { code: 401 | 403, .. } => true,
    OciDistributionError::RegistryError { envelope, .. } => envelope
      .errors
      .iter()
      .any(|e| matches!(e.code, OciErrorCode::Unauthorized | OciErrorCode::Denied)),
    _ => false,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use oci_client::secrets::RegistryAuth;

  fn write(path: &std::path::Path, contents: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
  }

  #[test]
  fn reads_inline_docker_basic_auth() {
    let home = tempfile::tempdir().unwrap();
    write(
      &home.path().join(".docker/config.json"),
      r#"{"auths":{"registry.example.com":{"auth":"YWxpY2U6czNjcmV0"}}}"#,
    );
    let auth = resolve_auth("registry.example.com", home.path());
    assert!(
      matches!(&auth, RegistryAuth::Basic(u, p) if u == "alice" && p == "s3cret"),
      "{auth:?}"
    );
  }

  #[test]
  fn docker_takes_precedence_over_moonlit() {
    let home = tempfile::tempdir().unwrap();
    write(
      &home.path().join(".docker/config.json"),
      r#"{"auths":{"reg.example.com":{"auth":"YWxpY2U6czNjcmV0"}}}"#,
    );
    write(
      &home.path().join(".config/moonlit/credentials.toml"),
      "[registries.\"reg.example.com\"]\ntoken = \"tok\"\n",
    );
    assert!(matches!(
      resolve_auth("reg.example.com", home.path()),
      RegistryAuth::Basic(_, _)
    ));
  }

  #[test]
  fn reads_moonlit_bearer_token_when_no_docker_entry() {
    let home = tempfile::tempdir().unwrap();
    write(
      &home.path().join(".config/moonlit/credentials.toml"),
      "[registries.\"registry.moonlit.rs\"]\ntoken = \"abc123\"\n",
    );
    let auth = resolve_auth("registry.moonlit.rs", home.path());
    assert!(matches!(&auth, RegistryAuth::Bearer(t) if t == "abc123"), "{auth:?}");
  }

  #[test]
  fn reads_moonlit_basic_when_username_password_present() {
    let home = tempfile::tempdir().unwrap();
    write(
      &home.path().join(".config/moonlit/credentials.toml"),
      "[registries.\"reg.example.com\"]\nusername = \"bob\"\npassword = \"pw\"\n",
    );
    let auth = resolve_auth("reg.example.com", home.path());
    assert!(
      matches!(&auth, RegistryAuth::Basic(u, p) if u == "bob" && p == "pw"),
      "{auth:?}"
    );
  }

  #[test]
  fn anonymous_when_no_credentials_match() {
    let home = tempfile::tempdir().unwrap();
    assert!(matches!(resolve_auth("ghcr.io", home.path()), RegistryAuth::Anonymous));
  }

  #[test]
  fn ignores_docker_creds_store_entry() {
    let home = tempfile::tempdir().unwrap();
    write(
      &home.path().join(".docker/config.json"),
      r#"{"credsStore":"desktop","auths":{}}"#,
    );
    assert!(matches!(
      resolve_auth("reg.example.com", home.path()),
      RegistryAuth::Anonymous
    ));
  }

  #[test]
  fn a_moonlit_entry_without_a_token_or_full_basic_pair_is_anonymous() {
    let home = tempfile::tempdir().unwrap();
    write(
      &home.path().join(".config/moonlit/credentials.toml"),
      "[registries.\"reg.example.com\"]\nusername = \"bob\"\n",
    );
    assert!(matches!(
      resolve_auth("reg.example.com", home.path()),
      RegistryAuth::Anonymous
    ));
  }
}
