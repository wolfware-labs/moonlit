use serde::Deserialize;

pub enum Credential {
    Basic { username: String, password: String },
    Bearer { token: String },
}

#[derive(Deserialize)]
pub struct AuthorizeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Deserialize)]
pub struct TokenSuccess {
    pub access_token: String,
}

#[derive(Deserialize)]
pub struct TokenError {
    pub error: String,
}

pub enum PollResponse {
    Pending,
    SlowDown,
    Denied,
    Expired,
    InvalidGrant,
    Approved { access_token: String },
}

pub enum PollDecision {
    KeepWaiting { interval: u64 },
    Done(String),
    Fail(&'static str),
}
