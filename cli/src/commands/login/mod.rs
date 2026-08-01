//! `moonlit login [host]` — authenticate to an OCI registry.
//!
//! By default this runs the RFC 8628 device-authorization flow ([`device`]): request a device code,
//! open the browser to the approval page, and store the minted PAT as a Bearer credential in
//! `~/.config/moonlit/credentials.toml`. An explicit `--token`/`--username` takes the manual path
//! (for CI), storing exactly what the caller supplies.

use std::path::Path;

use crate::cli::LoginArgs;

pub(crate) mod device;

/// A credential to persist for one registry host.
pub(crate) enum Credential {
    Basic { username: String, password: String },
    Bearer { token: String },
}

/// Upsert a registry credential into `credentials.toml` under `home`, preserving other hosts.
/// Writes the file with `0600` permissions on unix.
pub(crate) fn write_credential(home: &Path, host: &str, cred: &Credential) -> std::io::Result<()> {
    let path = home.join(".config/moonlit/credentials.toml");
    let mut doc: toml::Table = std::fs::read_to_string(&path)
        .ok()
        .and_then(|t| t.parse().ok())
        .unwrap_or_default();

    let registries = doc
        .entry("registries".to_string())
        .or_insert_with(|| toml::Value::Table(toml::Table::new()));
    if !registries.is_table() {
        *registries = toml::Value::Table(toml::Table::new());
    }
    let registries = registries.as_table_mut().expect("registries is a table");

    let mut entry = toml::Table::new();
    match cred {
        Credential::Basic { username, password } => {
            entry.insert("username".into(), username.clone().into());
            entry.insert("password".into(), password.clone().into());
        }
        Credential::Bearer { token } => {
            entry.insert("token".into(), token.clone().into());
        }
    }
    registries.insert(host.to_string(), toml::Value::Table(entry));

    let text = toml::to_string_pretty(&doc).map_err(std::io::Error::other)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Create the file with 0600 from the outset on unix (no world/group-readable window for the
    // plaintext token); still reset perms afterward so a pre-existing looser file is tightened too.
    #[cfg(unix)]
    {
        use std::io::Write as _;
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&path)?;
        f.write_all(text.as_bytes())?;
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    #[cfg(not(unix))]
    {
        std::fs::write(&path, text)?;
    }
    Ok(())
}

pub async fn run(args: LoginArgs) -> i32 {
    // Manual path (CI / non-device registries): any explicit --token or --username bypasses the
    // browser device flow and stores exactly what the caller supplied.
    if args.token.is_some() || args.username.is_some() {
        return run_manual(args);
    }
    device::login(args.host).await
}

/// Store a credential supplied directly by the caller (`--token`/`--username`, or interactive
/// prompts). No browser flow; used for CI and registries that are not Moonlit's device endpoint.
fn run_manual(args: LoginArgs) -> i32 {
    let host = args
        .host
        .unwrap_or_else(|| crate::cli::DEFAULT_REGISTRY_HOST.to_string());
    let interactive = std::io::IsTerminal::is_terminal(&std::io::stdin());

    let username = match args.username {
        Some(u) => Some(u),
        None if interactive => cliclack::input("Username (leave blank for token-only)")
            .required(false)
            .interact()
            .ok(),
        None => None,
    };
    let token = match args.token {
        Some(t) => t,
        None if interactive => match cliclack::password("Token").interact() {
            Ok(t) => t,
            Err(_) => {
                eprintln!("error: login cancelled");
                return 2;
            }
        },
        None => {
            eprintln!("error: login requires --token in a non-interactive terminal");
            return 2;
        }
    };
    if token.is_empty() {
        eprintln!("error: token must not be empty");
        return 2;
    }

    let cred = match username {
        Some(u) if !u.is_empty() => Credential::Basic {
            username: u,
            password: token,
        },
        _ => Credential::Bearer { token },
    };

    let home = dirs::home_dir().unwrap_or_default();
    match write_credential(&home, &host, &cred) {
        Ok(()) => {
            println!("Logged in to {host}.");
            0
        }
        Err(e) => {
            eprintln!("error: failed to write credentials: {e}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(home: &Path) -> toml::Table {
        let text = std::fs::read_to_string(home.join(".config/moonlit/credentials.toml")).unwrap();
        text.parse().unwrap()
    }

    #[test]
    fn writes_basic_credentials() {
        let home = tempfile::tempdir().unwrap();
        write_credential(
            home.path(),
            "ghcr.io",
            &Credential::Basic {
                username: "alice".into(),
                password: "pat".into(),
            },
        )
        .unwrap();
        let doc = read(home.path());
        let e = &doc["registries"]["ghcr.io"];
        assert_eq!(e["username"].as_str(), Some("alice"));
        assert_eq!(e["password"].as_str(), Some("pat"));
        assert!(e.get("token").is_none());
    }

    #[test]
    fn writes_bearer_credentials() {
        let home = tempfile::tempdir().unwrap();
        write_credential(
            home.path(),
            "registry.moonlitbuild.dev",
            &Credential::Bearer {
                token: "abc123".into(),
            },
        )
        .unwrap();
        let doc = read(home.path());
        let e = &doc["registries"]["registry.moonlitbuild.dev"];
        assert_eq!(e["token"].as_str(), Some("abc123"));
        assert!(e.get("username").is_none());
    }

    #[test]
    fn upsert_preserves_other_hosts_and_overwrites_same_host() {
        let home = tempfile::tempdir().unwrap();
        write_credential(
            home.path(),
            "a.example.com",
            &Credential::Bearer { token: "ta".into() },
        )
        .unwrap();
        write_credential(
            home.path(),
            "b.example.com",
            &Credential::Bearer { token: "tb".into() },
        )
        .unwrap();
        // Overwrite host a.
        write_credential(
            home.path(),
            "a.example.com",
            &Credential::Bearer {
                token: "ta2".into(),
            },
        )
        .unwrap();
        let doc = read(home.path());
        assert_eq!(
            doc["registries"]["a.example.com"]["token"].as_str(),
            Some("ta2")
        );
        assert_eq!(
            doc["registries"]["b.example.com"]["token"].as_str(),
            Some("tb")
        );
    }

    #[cfg(unix)]
    #[test]
    fn credentials_file_is_0600() {
        use std::os::unix::fs::PermissionsExt;
        let home = tempfile::tempdir().unwrap();
        write_credential(
            home.path(),
            "ghcr.io",
            &Credential::Bearer { token: "t".into() },
        )
        .unwrap();
        let mode = std::fs::metadata(home.path().join(".config/moonlit/credentials.toml"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}
