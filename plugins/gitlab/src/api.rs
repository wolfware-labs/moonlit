//! Thin GitLab REST client over `ctx.http()`: PRIVATE-TOKEN auth, Link-header
//! pagination, and status→error mapping. The API base is dynamic (self-hosted
//! GitLab via `baseUrl`), so callers pass `api_base` (e.g. `https://gitlab.com/api/v4`).

use moonlit_sdk::http::{Request, Response};
use moonlit_sdk::prelude::*;
use serde_json::Value;

fn auth<'a>(req: Request<'a>, token: &str) -> Request<'a> {
    req.header("PRIVATE-TOKEN", token)
        .header("User-Agent", "moonlit")
        .header("Accept", "application/json")
}

fn check_status(resp: &Response) -> Result<(), String> {
    if resp.is_success() {
        return Ok(());
    }
    let status = resp.status();
    if status == 401 || status == 403 {
        return Err(format!(
            "GitLab authentication failed (HTTP {status}). Check the configured token."
        ));
    }
    let body = resp.text().unwrap_or_default();
    Err(format!("GitLab API request failed (HTTP {status}): {body}"))
}

/// The `rel="next"` URL from a `Link` header, if present.
fn next_link(resp: &Response) -> Option<String> {
    let link = resp.header("link")?;
    for part in link.split(',') {
        let seg = part.trim();
        if seg.contains("rel=\"next\"") {
            let start = seg.find('<')?;
            let end = seg.find('>')?;
            return Some(seg[start + 1..end].to_string());
        }
    }
    None
}

/// GET `path` (relative to `api_base`), following pagination. Accumulates each
/// page's JSON array.
pub fn get_paginated(
    ctx: &Context,
    api_base: &str,
    token: &str,
    path: &str,
) -> Result<Vec<Value>, String> {
    let sep = if path.contains('?') { '&' } else { '?' };
    let mut url = format!("{api_base}{path}{sep}per_page=100");
    let mut out = Vec::new();
    loop {
        let resp = auth(ctx.http().get(url.as_str()), token).send()?;
        check_status(&resp)?;
        let page: Vec<Value> = resp.json()?;
        out.extend(page);
        match next_link(&resp) {
            Some(next) => url = next,
            None => break,
        }
    }
    Ok(out)
}

/// POST `body` to `path` (relative to `api_base`).
pub fn post_json(
    ctx: &Context,
    api_base: &str,
    token: &str,
    path: &str,
    body: &Value,
) -> Result<Response, String> {
    let resp = auth(ctx.http().post(format!("{api_base}{path}")), token)
        .json(body)
        .send()?;
    check_status(&resp)?;
    Ok(resp)
}

/// PUT `path` (relative to `api_base`) with no body — used for `?add_labels=`.
pub fn put(ctx: &Context, api_base: &str, token: &str, path: &str) -> Result<Response, String> {
    let resp = auth(ctx.http().put(format!("{api_base}{path}")), token).send()?;
    check_status(&resp)?;
    Ok(resp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_sdk::http::HttpMethod;
    use moonlit_sdk::testing::MockHost;

    const BASE: &str = "https://gitlab.com/api/v4";

    #[test]
    fn get_attaches_private_token_and_per_page() {
        let host = MockHost::new().with_http_response(200, b"[]");
        let ctx = Context::new(&host, "/w".into(), "s".into());
        let _ = get_paginated(
            &ctx,
            BASE,
            "tok",
            "/projects/o%2Fr/merge_requests?state=merged",
        )
        .unwrap();
        let reqs = host.recorded_requests();
        assert_eq!(reqs[0].authority, "gitlab.com");
        assert_eq!(
            reqs[0].path_with_query,
            "/api/v4/projects/o%2Fr/merge_requests?state=merged&per_page=100"
        );
        let has = |k: &str, v: &str| {
            reqs[0]
                .headers
                .iter()
                .any(|(hk, hv)| hk.eq_ignore_ascii_case(k) && hv == v)
        };
        assert!(has("private-token", "tok"));
        assert!(has("user-agent", "moonlit"));
        assert!(has("accept", "application/json"));
    }

    #[test]
    fn follows_link_header_to_second_page() {
        let host = MockHost::new()
            .with_http_response_headers(
                200,
                vec![(
                    "link".into(),
                    "<https://gitlab.com/api/v4/x?page=2>; rel=\"next\"".into(),
                )],
                b"[{\"iid\":1}]",
            )
            .with_http_response(200, b"[{\"iid\":2}]");
        let ctx = Context::new(&host, "/w".into(), "s".into());
        let items = get_paginated(&ctx, BASE, "tok", "/x").unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(host.recorded_requests().len(), 2);
        assert_eq!(
            host.recorded_requests()[1].path_with_query,
            "/api/v4/x?page=2"
        );
    }

    #[test]
    fn auth_error_maps_401() {
        let host = MockHost::new().with_http_response(401, b"{}");
        let ctx = Context::new(&host, "/w".into(), "s".into());
        let e = match get_paginated(&ctx, BASE, "tok", "/x") {
            Ok(_) => panic!("401 must be an error"),
            Err(e) => e,
        };
        assert_eq!(
            e,
            "GitLab authentication failed (HTTP 401). Check the configured token."
        );
    }

    #[test]
    fn server_error_maps_with_body() {
        let host = MockHost::new().with_http_response(500, b"boom");
        let ctx = Context::new(&host, "/w".into(), "s".into());
        let e = match put(&ctx, BASE, "tok", "/x") {
            Ok(_) => panic!("500 must be an error"),
            Err(e) => e,
        };
        assert_eq!(e, "GitLab API request failed (HTTP 500): boom");
    }

    #[test]
    fn post_sends_json_body() {
        let host = MockHost::new().with_http_response(201, br#"{"name":"1.0"}"#);
        let ctx = Context::new(&host, "/w".into(), "s".into());
        let r = post_json(
            &ctx,
            BASE,
            "tok",
            "/projects/o%2Fr/releases",
            &serde_json::json!({"name":"1.0"}),
        )
        .unwrap();
        assert!(r.is_success());
        let reqs = host.recorded_requests();
        assert_eq!(reqs[0].method, HttpMethod::Post);
        assert_eq!(reqs[0].path_with_query, "/api/v4/projects/o%2Fr/releases");
        assert_eq!(reqs[0].body.as_deref().unwrap(), br#"{"name":"1.0"}"#);
    }

    #[test]
    fn put_sends_request_to_path() {
        let host = MockHost::new().with_http_response(200, b"{}");
        let ctx = Context::new(&host, "/w".into(), "s".into());
        let r = put(
            &ctx,
            BASE,
            "tok",
            "/projects/o%2Fr/issues/9?add_labels=released",
        )
        .unwrap();
        assert!(r.is_success());
        let reqs = host.recorded_requests();
        assert_eq!(reqs[0].method, HttpMethod::Put);
        assert_eq!(
            reqs[0].path_with_query,
            "/api/v4/projects/o%2Fr/issues/9?add_labels=released"
        );
    }
}
