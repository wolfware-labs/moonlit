pub(crate) mod device;
mod model;

use crate::cli::LoginArgs;
use crate::commands::login::model::Credential;
use std::path::Path;

pub async fn run(args: LoginArgs) -> i32 {
    if args.token.is_some() || args.username.is_some() {
        return run_manual(args);
    }
    device::login(args.host).await
}

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

    let Some(home) = home_dir() else {
        eprintln!("error: could not determine your home directory (is $HOME set?)");
        return 1;
    };
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

fn write_credential(home: &Path, host: &str, cred: &Credential) -> std::io::Result<()> {
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
    write_doc_0600(&path, &text)
}

fn home_dir() -> Option<std::path::PathBuf> {
    dirs::home_dir().filter(|p| !p.as_os_str().is_empty())
}

fn write_doc_0600(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    #[cfg(unix)]
    {
        use std::io::Write as _;
        use std::os::unix::fs::OpenOptionsExt;
        let mut tmp_os = path.as_os_str().to_os_string();
        tmp_os.push(".tmp");
        let tmp = std::path::PathBuf::from(tmp_os);
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&tmp)?;
        f.write_all(text.as_bytes())?;
        f.sync_all()?;
        std::fs::rename(&tmp, path)?;
    }
    #[cfg(not(unix))]
    {
        std::fs::write(path, text)?;
    }
    Ok(())
}

fn read_bearer(home: &Path, host: &str) -> Option<String> {
    let path = home.join(".config/moonlit/credentials.toml");
    let doc: toml::Table = std::fs::read_to_string(&path).ok()?.parse().ok()?;
    doc.get("registries")?
        .as_table()?
        .get(host)?
        .as_table()?
        .get("token")?
        .as_str()
        .map(str::to_string)
}

fn remove_credential(home: &Path, host: &str) -> std::io::Result<bool> {
    let path = home.join(".config/moonlit/credentials.toml");
    let mut doc: toml::Table = match std::fs::read_to_string(&path) {
        Ok(t) => t.parse().unwrap_or_default(),
        Err(_) => return Ok(false),
    };

    let Some(registries) = doc.get_mut("registries").and_then(|r| r.as_table_mut()) else {
        return Ok(false);
    };
    if registries.remove(host).is_none() {
        return Ok(false);
    }

    let text = toml::to_string_pretty(&doc).map_err(std::io::Error::other)?;
    write_doc_0600(&path, &text)?;
    Ok(true)
}
