//! Outgoing-HTTP authorization: allow/deny each wasi:http request by its authority
//! (host), matched against the plugin's `network` allowlist globs.

use globset::GlobSet;
use wasmtime_wasi_http::p2::{HttpResult, WasiHttpHooks, default_send_request};

use crate::config::model::Permissions;
use crate::host::perms::network_globset;

pub struct AllowlistHooks {
    allowed: GlobSet,
}

impl AllowlistHooks {
    pub fn from_permissions(p: &Permissions) -> Self {
        Self {
            allowed: network_globset(&p.network),
        }
    }
}

impl WasiHttpHooks for AllowlistHooks {
    fn send_request(
        &mut self,
        request: hyper::Request<wasmtime_wasi_http::p2::body::HyperOutgoingBody>,
        config: wasmtime_wasi_http::p2::types::OutgoingRequestConfig,
    ) -> HttpResult<wasmtime_wasi_http::p2::types::HostFutureIncomingResponse> {
        let host = request.uri().host().unwrap_or_default().to_string();
        if self.allowed.is_match(&host) {
            Ok(default_send_request(request, config))
        } else {
            Err(wasmtime_wasi_http::p2::bindings::http::types::ErrorCode::HttpRequestDenied.into())
        }
    }
}
