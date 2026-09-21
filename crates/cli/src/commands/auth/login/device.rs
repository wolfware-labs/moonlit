use crate::cli::DEFAULT_REGISTRY_HOST;
use crate::commands::auth::login::model::{
    AuthorizeResponse, Credential, PollDecision, PollResponse, TokenError, TokenSuccess,
};
use crate::commands::auth::{REQUEST_TIMEOUT, base_url, home_dir, http_client};
use std::time::Duration;

pub async fn login(host_arg: Option<String>) -> i32 {
    let host = host_arg.unwrap_or_else(|| DEFAULT_REGISTRY_HOST.to_string());
    let base = base_url(&host);
    let http = match http_client(REQUEST_TIMEOUT) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: could not initialize the HTTP client: {e}");
            return 1;
        }
    };

    let authorize: AuthorizeResponse = match http
        .post(format!("{base}/api/v1/device/authorize"))
        .json(&serde_json::json!({ "clientName": client_name() }))
        .send()
        .await
        .and_then(|r| r.error_for_status())
    {
        Ok(r) => match r.json().await {
            Ok(a) => a,
            Err(e) => {
                eprintln!("error: bad response from {host}: {e}");
                return 1;
            }
        },
        Err(e) => {
            eprintln!("error: could not reach {host}: {e}");
            return 1;
        }
    };

    println!("First copy your one-time code: {}", authorize.user_code);
    println!("Opening {} …", authorize.verification_uri);
    let opened = opens_safely(&authorize.verification_uri_complete)
        && open::that(&authorize.verification_uri_complete).is_ok();
    if !opened {
        println!(
            "Could not open a browser. Visit {} and enter the code above.",
            authorize.verification_uri
        );
    }

    let mut interval = authorize.interval.max(1);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(authorize.expires_in.max(1));
    let mut consecutive_errors = 0u32;
    const MAX_CONSECUTIVE_ERRORS: u32 = 5;
    let spinner = cliclack::spinner();
    spinner.start("Waiting for authorization…");
    let token = loop {
        tokio::time::sleep(Duration::from_secs(interval)).await;
        if tokio::time::Instant::now() >= deadline {
            spinner.error("timed out");
            eprintln!("error: login timed out; run `moonlit login` again");
            return 1;
        }
        let resp = match poll_once(&http, &base, &authorize.device_code).await {
            Ok(r) => {
                consecutive_errors = 0;
                r
            }
            Err(e) => {
                consecutive_errors += 1;
                if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                    spinner.error("network error");
                    eprintln!("error: {e}");
                    return 1;
                }
                continue;
            }
        };
        match decide(resp, interval) {
            PollDecision::KeepWaiting { interval: next } => interval = next,
            PollDecision::Done(t) => break t,
            PollDecision::Fail(msg) => {
                spinner.error("failed");
                eprintln!("error: {msg}");
                return 1;
            }
        }
    };
    spinner.stop("Authorized.");

    let Some(home) = home_dir() else {
        eprintln!("error: could not determine your home directory (is $HOME set?)");
        return 1;
    };
    match super::write_credential(&home, &host, &Credential::Bearer { token }) {
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

async fn poll_once(
    http: &reqwest::Client,
    base: &str,
    device_code: &str,
) -> reqwest::Result<PollResponse> {
    let resp = http
        .post(format!("{base}/api/v1/device/token"))
        .json(&serde_json::json!({ "deviceCode": device_code }))
        .send()
        .await?;

    if resp.status().is_success() {
        let ok: TokenSuccess = resp.json().await?;
        return Ok(PollResponse::Approved {
            access_token: ok.access_token,
        });
    }

    let err: TokenError = resp.json().await?;
    Ok(match err.error.as_str() {
        "authorization_pending" => PollResponse::Pending,
        "slow_down" => PollResponse::SlowDown,
        "access_denied" => PollResponse::Denied,
        "expired_token" => PollResponse::Expired,
        _ => PollResponse::InvalidGrant,
    })
}

fn decide(resp: PollResponse, interval: u64) -> PollDecision {
    match resp {
        PollResponse::Pending => PollDecision::KeepWaiting { interval },
        PollResponse::SlowDown => PollDecision::KeepWaiting {
            interval: interval + 5,
        },
        PollResponse::Approved { access_token } => PollDecision::Done(access_token),
        PollResponse::Denied => PollDecision::Fail("authorization denied"),
        PollResponse::Expired => PollDecision::Fail("login timed out; run `moonlit login` again"),
        PollResponse::InvalidGrant => PollDecision::Fail("invalid device code"),
    }
}

fn opens_safely(url: &str) -> bool {
    url.starts_with("https://") || url.starts_with("http://")
}

fn client_name() -> String {
    let host = gethostname::gethostname().to_string_lossy().to_string();
    format!("Moonlit CLI — {host}")
}
