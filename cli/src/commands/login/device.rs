//! `moonlit login` device-authorization flow (RFC 8628): request a device code, open the browser to
//! the approval page, poll until the registry mints a PAT, and store it as a Bearer credential.
//!
//! The poll loop is split into a pure state-machine step ([`decide`]) and the async I/O glue
//! ([`login`]) so the RFC 8628 transitions can be unit-tested without HTTP or a running registry.

use std::time::Duration;

use serde::Deserialize;

use crate::cli::DEFAULT_REGISTRY_HOST;

/// Reply to `POST /api/v1/device/authorize` (registry emits RFC 8628 snake_case fields).
#[derive(Deserialize)]
struct AuthorizeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    verification_uri_complete: String,
    #[allow(dead_code)]
    expires_in: u64,
    interval: u64,
}

/// Success reply to `POST /api/v1/device/token` (200).
#[derive(Deserialize)]
struct TokenSuccess {
    access_token: String,
}

/// Error reply to `POST /api/v1/device/token` (400) — an RFC 8628 error string.
#[derive(Deserialize)]
struct TokenError {
    error: String,
}

/// A parsed `/token` poll reply, normalized to the RFC 8628 outcomes we act on.
pub enum PollResponse {
    Pending,
    SlowDown,
    Denied,
    Expired,
    InvalidGrant,
    Approved { access_token: String },
}

/// The next action after a poll: wait again (possibly slower), finish with a token, or give up.
pub enum PollDecision {
    KeepWaiting { interval: u64 },
    Done(String),
    Fail(&'static str),
}

/// Pure state-machine step: map a poll reply + current interval to the next action.
/// `slow_down` bumps the interval by 5s per RFC 8628 §3.5.
pub fn decide(resp: PollResponse, interval: u64) -> PollDecision {
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

/// Registry base URL: plain `http` only for loopback hosts, `https` otherwise.
pub(crate) fn base_url(host: &str) -> String {
    let scheme = if is_loopback(host) { "http" } else { "https" };
    format!("{scheme}://{host}")
}

/// Whether `host` (optionally `host:port`) names the loopback interface. The hostname is matched
/// EXACTLY: a `starts_with` test would treat `localhost.evil.com` / `127.0.0.1.attacker.com` as
/// local and silently downgrade those connections to cleartext `http`, exposing the token.
fn is_loopback(host: &str) -> bool {
    let hostname = if let Some(rest) = host.strip_prefix('[') {
        // Bracketed IPv6 literal, e.g. `[::1]` or `[::1]:5185`.
        rest.split(']').next().unwrap_or(rest)
    } else {
        // `host` or `host:port`: strip only a trailing numeric port, and never split a bare IPv6
        // literal (which itself contains ':').
        match host.rsplit_once(':') {
            Some((h, port))
                if !h.contains(':')
                    && !port.is_empty()
                    && port.bytes().all(|b| b.is_ascii_digit()) =>
            {
                h
            }
            _ => host,
        }
    };
    hostname.eq_ignore_ascii_case("localhost") || hostname == "127.0.0.1" || hostname == "::1"
}

/// The token label shown in the portal, e.g. `Moonlit CLI — my-laptop`.
fn client_name() -> String {
    let host = gethostname::gethostname().to_string_lossy().to_string();
    format!("Moonlit CLI — {host}")
}

/// Run the full device-authorization flow and store the minted PAT as a Bearer credential.
/// Returns a process exit code.
pub async fn login(host_arg: Option<String>) -> i32 {
    let host = host_arg.unwrap_or_else(|| DEFAULT_REGISTRY_HOST.to_string());
    let base = base_url(&host);
    let http = reqwest::Client::new();

    // 1. Request a device code.
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

    // 2. Show the code and open the browser.
    println!("First copy your one-time code: {}", authorize.user_code);
    println!("Opening {} …", authorize.verification_uri);
    if open::that(&authorize.verification_uri_complete).is_err() {
        println!(
            "Could not open a browser. Visit {} and enter the code above.",
            authorize.verification_uri
        );
    }

    // 3. Poll until approved, denied, or expired.
    let mut interval = authorize.interval.max(1);
    let spinner = cliclack::spinner();
    spinner.start("Waiting for authorization…");
    let token = loop {
        tokio::time::sleep(Duration::from_secs(interval)).await;
        let resp = match poll_once(&http, &base, &authorize.device_code).await {
            Ok(r) => r,
            Err(e) => {
                spinner.error("network error");
                eprintln!("error: {e}");
                return 1;
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

    // 4. Store as Bearer (existing 0600 writer).
    let home = dirs::home_dir().unwrap_or_default();
    match super::write_credential(&home, &host, &super::Credential::Bearer { token }) {
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

/// One `POST /api/v1/device/token` round-trip, parsed into a [`PollResponse`].
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_keeps_waiting_same_interval() {
        match decide(PollResponse::Pending, 5) {
            PollDecision::KeepWaiting { interval } => assert_eq!(interval, 5),
            _ => panic!("expected KeepWaiting"),
        }
    }

    #[test]
    fn slow_down_increases_interval_by_5() {
        match decide(PollResponse::SlowDown, 5) {
            PollDecision::KeepWaiting { interval } => assert_eq!(interval, 10),
            _ => panic!("expected KeepWaiting with bumped interval"),
        }
    }

    #[test]
    fn approved_is_done_with_token() {
        match decide(
            PollResponse::Approved {
                access_token: "mlp_x".into(),
            },
            5,
        ) {
            PollDecision::Done(t) => assert_eq!(t, "mlp_x"),
            _ => panic!("expected Done"),
        }
    }

    #[test]
    fn denied_and_expired_and_invalid_fail() {
        assert!(matches!(
            decide(PollResponse::Denied, 5),
            PollDecision::Fail(_)
        ));
        assert!(matches!(
            decide(PollResponse::Expired, 5),
            PollDecision::Fail(_)
        ));
        assert!(matches!(
            decide(PollResponse::InvalidGrant, 5),
            PollDecision::Fail(_)
        ));
    }

    #[test]
    fn base_url_uses_http_for_localhost_https_otherwise() {
        // Loopback → http (with or without a port, IPv4 and bracketed IPv6).
        assert!(base_url("localhost").starts_with("http://"));
        assert!(base_url("localhost:5185").starts_with("http://"));
        assert!(base_url("127.0.0.1:5185").starts_with("http://"));
        assert!(base_url("[::1]:5185").starts_with("http://"));
        assert!(base_url("::1").starts_with("http://"));
        // Real hosts → https.
        assert!(base_url("registry.moonlitbuild.dev").starts_with("https://"));
        assert!(base_url("registry.moonlitbuild.dev:443").starts_with("https://"));
    }

    #[test]
    fn base_url_does_not_downgrade_loopback_lookalike_hosts() {
        // A prefix check would wrongly send these over cleartext http; an exact match must not.
        assert!(base_url("localhost.evil.com").starts_with("https://"));
        assert!(base_url("127.0.0.1.attacker.com").starts_with("https://"));
        assert!(base_url("localhostapi.internal").starts_with("https://"));
        assert!(base_url("localhost.evil.com:8080").starts_with("https://"));
    }
}
