//! Outgoing-HTTP authorization: allow/deny each wasi:http request by its authority
//! (host), matched against the plugin's `network` allowlist globs.

use globset::GlobSet;
use wasmtime_wasi_http::p2::{HttpResult, WasiHttpHooks, default_send_request};

use crate::config::model::Permissions;
use crate::host::perms::network_globset;

pub struct AllowlistHooks {
    allowed: GlobSet,
    events: std::sync::Arc<dyn crate::host::HostEventSink>,
}

impl AllowlistHooks {
    pub fn new(p: &Permissions, events: std::sync::Arc<dyn crate::host::HostEventSink>) -> Self {
        Self {
            allowed: network_globset(&p.network),
            events,
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
            self.events.log(
                "",
                crate::host::LogLevel::Warn,
                &format!(
                    "blocked from connecting to '{host}' — add it to the plugin's permissions.network"
                ),
            );
            Err(wasmtime_wasi_http::p2::bindings::http::types::ErrorCode::HttpRequestDenied.into())
        }
    }
}
