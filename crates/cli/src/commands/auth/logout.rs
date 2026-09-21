use crate::cli::{DEFAULT_REGISTRY_HOST, LogoutArgs};
use crate::commands::auth::{
    REQUEST_TIMEOUT, base_url, home_dir, http_client, read_bearer, write_doc_0600,
};
use std::path::Path;

pub async fn run(args: LogoutArgs) -> i32 {
    let host = args
        .host
        .unwrap_or_else(|| DEFAULT_REGISTRY_HOST.to_string());
    let Some(home) = home_dir() else {
        eprintln!("error: could not determine your home directory (is $HOME set?)");
        return 1;
    };

    if !args.local
        && let Some(token) = read_bearer(&home, &host)
    {
        let base = base_url(&host);
        let revoked = match http_client(REQUEST_TIMEOUT) {
            Ok(http) => http
                .post(format!("{base}/api/v1/device/logout"))
                .bearer_auth(&token)
                .send()
                .await
                .and_then(|r| r.error_for_status())
                .is_ok(),
            Err(_) => false,
        };
        if !revoked {
            eprintln!(
                "warning: could not revoke the token on {host} (revoke it in the portal); \
                 removing it locally anyway."
            );
        }
    }

    match remove_credential(&home, &host) {
        Ok(true) => {
            println!("Logged out of {host}.");
            0
        }
        Ok(false) => {
            println!("Not logged in to {host}.");
            0
        }
        Err(e) => {
            eprintln!("error: failed to update credentials: {e}");
            1
        }
    }
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
