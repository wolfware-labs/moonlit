use crate::cli::{DEFAULT_REGISTRY_HOST, LogoutArgs};
use crate::commands::login::{device, home_dir, read_bearer, remove_credential};

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
        let base = device::base_url(&host);
        let revoked = match device::http_client(device::REQUEST_TIMEOUT) {
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
