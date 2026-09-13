use globset::GlobSet;
use http_body_util::BodyExt;
use hyper::http;
use std::future::Future;
use wasmtime_wasi_http::{Error, RequestOptions, WasiBody, WasiHttpHooks, default_send_request};

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

type SendResult = Box<
    dyn Future<
            Output = Result<
                (
                    http::Response<WasiBody>,
                    Box<dyn Future<Output = Result<(), Error>> + Send>,
                ),
                Error,
            >,
        > + Send,
>;

impl WasiHttpHooks for AllowlistHooks {
    fn send_request(
        &mut self,
        request: http::Request<WasiBody>,
        options: Option<RequestOptions>,
        fut: Box<dyn Future<Output = Result<(), Error>> + Send>,
    ) -> SendResult {
        let host = request.uri().host().unwrap_or_default().to_string();

        if !self.allowed.is_match(&host) {
            self.events.log(
                "",
                crate::host::LogLevel::Warn,
                &format!(
                    "blocked from connecting to '{host}' — add it to the plugin's permissions.network"
                ),
            );
            return Box::new(async move { Err(Error::HttpRequestDenied) });
        }

        _ = fut;
        Box::new(async move {
            let (res, io) = default_send_request(request, options).await?;
            Ok((
                res.map(BodyExt::boxed_unsync),
                Box::new(io) as Box<dyn Future<Output = _> + Send>,
            ))
        })
    }
}
