//! The real host: implements `Host` against the generated wit-bindgen imports.
//! wasm-only — the native import stubs abort if called, so this whole module is
//! `cfg(target_arch = "wasm32")`. Binding-call shapes verified against wasm32-wasip2.

use crate::context::{Host, LogLevel};
use crate::http::{HttpMethod, HttpRequestData, HttpResponseData};
use crate::process::{ChildHandle, OutputChunk, ProcessCommand, ProcessOutput, StdioStream};

/// The real host, backed by the wit-bindgen imports.
pub struct RealHost;

impl Host for RealHost {
    fn log(&self, level: LogLevel, message: &str) {
        use crate::bindings::moonlit::plugin::types::LogLevel as W;
        let w = match level {
            LogLevel::Debug => W::Debug,
            LogLevel::Info => W::Info,
            LogLevel::Warn => W::Warn,
            LogLevel::Error => W::Error,
        };
        crate::bindings::moonlit::plugin::host::log(w, message);
    }
    fn get_config(&self, path: &str) -> Option<String> {
        crate::bindings::moonlit::plugin::host::get_config(path)
    }
    fn report_progress(&self, message: &str) {
        crate::bindings::moonlit::plugin::host::report_progress(message);
    }
    fn process_run(&self, cmd: &ProcessCommand) -> Result<ProcessOutput, String> {
        let (code, chunks) = crate::bindings::moonlit::plugin::process::run(&to_wit_command(cmd))?;
        Ok(ProcessOutput {
            exit_code: code,
            chunks: chunks.into_iter().map(from_wit_chunk).collect(),
        })
    }
    fn process_spawn(&self, cmd: &ProcessCommand) -> Result<Box<dyn ChildHandle>, String> {
        let child = crate::bindings::moonlit::plugin::process::spawn(&to_wit_command(cmd))?;
        Ok(Box::new(RealChild { child }))
    }
    fn http_send(&self, req: &HttpRequestData) -> Result<HttpResponseData, String> {
        http_send_impl(req)
    }
    fn env_var(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }
    fn env_vars(&self) -> Vec<(String, String)> {
        std::env::vars().collect()
    }
    fn random_bytes(&self, n: usize) -> Vec<u8> {
        crate::bindings::wasi::random::random::get_random_bytes(n as u64)
    }
    fn monotonic_nanos(&self) -> u64 {
        crate::bindings::wasi::clocks::monotonic_clock::now()
    }
    fn sleep_nanos(&self, nanos: u64) {
        crate::bindings::wasi::clocks::monotonic_clock::subscribe_duration(nanos).block();
    }
}

fn to_wit_command(cmd: &ProcessCommand) -> crate::bindings::moonlit::plugin::process::Command {
    crate::bindings::moonlit::plugin::process::Command {
        program: cmd.program.clone(),
        args: cmd.args.clone(),
        cwd: cmd.cwd.clone(),
        env: cmd.env.clone(),
        stdin: cmd.stdin.clone(),
    }
}

fn from_wit_chunk(c: crate::bindings::moonlit::plugin::process::OutputChunk) -> OutputChunk {
    let stream = match c.stream {
        crate::bindings::moonlit::plugin::process::StdioStream::Stdout => StdioStream::Stdout,
        crate::bindings::moonlit::plugin::process::StdioStream::Stderr => StdioStream::Stderr,
    };
    OutputChunk {
        stream,
        text: c.line,
    }
}

struct RealChild {
    child: crate::bindings::moonlit::plugin::process::Child,
}

impl ChildHandle for RealChild {
    fn next_line(&mut self) -> Option<OutputChunk> {
        self.child.next_line().map(from_wit_chunk)
    }
    fn wait(&mut self) -> i32 {
        self.child.wait()
    }
    fn kill(&mut self) {
        self.child.kill();
    }
}

fn http_send_impl(req: &HttpRequestData) -> Result<HttpResponseData, String> {
    use crate::bindings::wasi::http::outgoing_handler;
    use crate::bindings::wasi::http::types::{
        Fields, Method, OutgoingBody, OutgoingRequest, RequestOptions, Scheme,
    };
    use crate::bindings::wasi::io::streams::StreamError;

    let header_entries: Vec<(String, Vec<u8>)> = req
        .headers
        .iter()
        .map(|(k, v)| (k.clone(), v.as_bytes().to_vec()))
        .collect();
    let fields = Fields::from_list(&header_entries).map_err(|e| format!("headers: {e:?}"))?;
    let request = OutgoingRequest::new(fields);
    let method = match req.method {
        HttpMethod::Get => Method::Get,
        HttpMethod::Post => Method::Post,
        HttpMethod::Put => Method::Put,
        HttpMethod::Patch => Method::Patch,
        HttpMethod::Delete => Method::Delete,
    };
    request
        .set_method(&method)
        .map_err(|()| "set_method".to_string())?;
    let scheme = if req.scheme == "http" {
        Scheme::Http
    } else {
        Scheme::Https
    };
    request
        .set_scheme(Some(&scheme))
        .map_err(|()| "set_scheme".to_string())?;
    request
        .set_authority(Some(&req.authority))
        .map_err(|()| "set_authority".to_string())?;
    request
        .set_path_with_query(Some(&req.path_with_query))
        .map_err(|()| "set_path".to_string())?;

    let outgoing_body = request.body().map_err(|()| "body()".to_string())?;
    if let Some(bytes) = &req.body {
        let stream = outgoing_body
            .write()
            .map_err(|()| "body.write()".to_string())?;
        for chunk in bytes.chunks(4096) {
            stream
                .blocking_write_and_flush(chunk)
                .map_err(|e| format!("write: {e:?}"))?;
        }
        drop(stream);
    }
    OutgoingBody::finish(outgoing_body, None).map_err(|e| format!("body.finish: {e:?}"))?;

    let options = if let Some(ms) = req.timeout_ms {
        let opts = RequestOptions::new();
        let ns = ms.saturating_mul(1_000_000);
        opts.set_connect_timeout(Some(ns))
            .map_err(|()| "set_connect_timeout".to_string())?;
        opts.set_first_byte_timeout(Some(ns))
            .map_err(|()| "set_first_byte_timeout".to_string())?;
        Some(opts)
    } else {
        None
    };

    let future =
        outgoing_handler::handle(request, options).map_err(|e| format!("handle: {e:?}"))?;
    let pollable = future.subscribe();
    pollable.block();
    let response = future
        .get()
        .ok_or_else(|| "future not ready".to_string())?
        .map_err(|()| "future already taken".to_string())?
        .map_err(|e| format!("response: {e:?}"))?;

    let status = response.status();
    let headers = response
        .headers()
        .entries()
        .into_iter()
        .map(|(k, v)| (k, String::from_utf8_lossy(&v).into_owned()))
        .collect();

    let incoming_body = response.consume().map_err(|()| "consume".to_string())?;
    let body_stream = incoming_body
        .stream()
        .map_err(|()| "body.stream".to_string())?;
    let mut body = Vec::new();
    loop {
        match body_stream.blocking_read(8192) {
            Ok(chunk) => body.extend_from_slice(&chunk),
            Err(StreamError::Closed) => break,
            Err(e) => return Err(format!("body read: {e:?}")),
        }
    }
    Ok(HttpResponseData {
        status,
        headers,
        body,
    })
}
