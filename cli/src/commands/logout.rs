//! `moonlit logout [host]` — revoke the stored token server-side and remove the local credential.
//!
//! Revoke-then-remove by default; `--local` skips the server call. If the server cannot be reached
//! the local credential is still removed (with a warning) so the user is never wedged.

use crate::cli::{DEFAULT_REGISTRY_HOST, LogoutArgs};
use crate::commands::login::{device, read_bearer, remove_credential};

pub async fn run(args: LogoutArgs) -> i32 {
    let host = args
        .host
        .unwrap_or_else(|| DEFAULT_REGISTRY_HOST.to_string());
    let home = dirs::home_dir().unwrap_or_default();

    let Some(token) = read_bearer(&home, &host) else {
        println!("Not logged in to {host}.");
        return 0;
    };

    // Best-effort server-side revoke (unless --local). Never block local removal on it.
    if !args.local {
        let base = device::base_url(&host);
        let http = reqwest::Client::new();
        let revoked = http
            .post(format!("{base}/api/v1/device/logout"))
            .bearer_auth(&token)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .is_ok();
        if !revoked {
            eprintln!(
                "warning: removed local credentials; could not reach {host} to revoke the token \
                 (revoke it in the portal)."
            );
        }
    }

    match remove_credential(&home, &host) {
        Ok(_) => {
            println!("Logged out of {host}.");
            0
        }
        Err(e) => {
            eprintln!("error: failed to update credentials: {e}");
            1
        }
    }
}
