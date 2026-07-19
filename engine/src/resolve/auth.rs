//! OCI credential resolution (§8.3 step 5). Reads existing credentials only — the `moonlit login`
//! write flow is CLI territory. Precedence: `~/.docker/config.json` (inline `auth` entries only —
//! credential helpers/`credsStore` are not consulted) then `~/.config/moonlit/credentials.toml`,
//! falling back to anonymous.

use std::collections::HashMap;
use std::path::Path;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
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

/// Resolve credentials for `host`, reading credential files under `home`.
///
/// `pub(crate)` with no non-test caller until Task 8's dispatcher wires it in; the allow below
/// suppresses the interim dead-code lint (same pattern as `resolve_file`/`resolve_http`).
#[allow(dead_code)]
pub(crate) fn resolve_auth(host: &str, home: &Path) -> RegistryAuth {
    if let Some(auth) = docker_auth(host, home) {
        return auth;
    }
    if let Some(auth) = moonlit_auth(host, home) {
        return auth;
    }
    RegistryAuth::Anonymous
}

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
        // base64("alice:s3cret") = YWxpY2U6czNjcmV0
        write(
            &home.path().join(".docker/config.json"),
            r#"{"auths":{"registry.example.com":{"auth":"YWxpY2U6czNjcmV0"}}}"#,
        );
        match resolve_auth("registry.example.com", home.path()) {
            RegistryAuth::Basic(u, p) => {
                assert_eq!(u, "alice");
                assert_eq!(p, "s3cret");
            }
            other => panic!("expected Basic, got {other:?}"),
        }
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
            "[registries.\"registry.moonlitbuild.dev\"]\ntoken = \"abc123\"\n",
        );
        match resolve_auth("registry.moonlitbuild.dev", home.path()) {
            RegistryAuth::Bearer(t) => assert_eq!(t, "abc123"),
            other => panic!("expected Bearer, got {other:?}"),
        }
    }

    #[test]
    fn reads_moonlit_basic_when_username_password_present() {
        let home = tempfile::tempdir().unwrap();
        write(
            &home.path().join(".config/moonlit/credentials.toml"),
            "[registries.\"reg.example.com\"]\nusername = \"bob\"\npassword = \"pw\"\n",
        );
        match resolve_auth("reg.example.com", home.path()) {
            RegistryAuth::Basic(u, p) => {
                assert_eq!(u, "bob");
                assert_eq!(p, "pw");
            }
            other => panic!("expected Basic, got {other:?}"),
        }
    }

    #[test]
    fn anonymous_when_no_credentials_match() {
        let home = tempfile::tempdir().unwrap();
        assert!(matches!(
            resolve_auth("ghcr.io", home.path()),
            RegistryAuth::Anonymous
        ));
    }

    #[test]
    fn ignores_docker_creds_store_entry() {
        let home = tempfile::tempdir().unwrap();
        // A credsStore-only config has no inline `auth`; we do not shell out to helpers.
        write(
            &home.path().join(".docker/config.json"),
            r#"{"credsStore":"desktop","auths":{}}"#,
        );
        assert!(matches!(
            resolve_auth("reg.example.com", home.path()),
            RegistryAuth::Anonymous
        ));
    }
}
