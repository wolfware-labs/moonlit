//! Thin GitHub REST client over `ctx.http()`: canonical headers, Link-header
//! pagination, and status→error mapping.

use moonlit_sdk::http::{Request, Response};
use moonlit_sdk::prelude::*;
use serde_json::Value;

const BASE: &str = "https://api.github.com";

fn auth<'a>(req: Request<'a>, token: &str) -> Request<'a> {
    req.bearer(token)
        .header("User-Agent", "moonlit")
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
}

fn check_status(resp: &Response) -> Result<(), String> {
    if resp.is_success() {
        return Ok(());
    }
    let status = resp.status();
    if status == 401 || status == 403 {
        return Err(format!(
            "GitHub authentication failed (HTTP {status}). Check the configured token."
        ));
    }
    let body = resp.text().unwrap_or_default();
    Err(format!("GitHub API request failed (HTTP {status}): {body}"))
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

/// GET `path` (relative to the API base), following pagination. Accumulates
/// each page's JSON array.
pub fn get_paginated(ctx: &Context, token: &str, path: &str) -> Result<Vec<Value>, String> {
    let sep = if path.contains('?') { '&' } else { '?' };
    let mut url = format!("{BASE}{path}{sep}per_page=100");
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

/// POST `body` to `path` (relative to the API base).
pub fn post_json(ctx: &Context, token: &str, path: &str, body: &Value) -> Result<Response, String> {
    let url = format!("{BASE}{path}");
    let resp = auth(ctx.http().post(url.as_str()), token)
        .json(body)
        .send()?;
    check_status(&resp)?;
    Ok(resp)
}

/// GET a single JSON object (not paginated).
pub fn get_json(ctx: &Context, token: &str, path: &str) -> Result<Value, String> {
    let resp = auth(ctx.http().get(format!("{BASE}{path}")), token).send()?;
    check_status(&resp)?;
    resp.json::<Value>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_sdk::testing::MockHost;

    #[test]
    fn get_attaches_canonical_headers_and_per_page() {
        let host = MockHost::new().with_http_response(200, b"[]");
        let ctx = Context::new(&host, "/w".into(), "s".into());
        let _ = get_paginated(&ctx, "tok", "/repos/o/r/pulls?state=all").unwrap();
        let reqs = host.recorded_requests();
        assert_eq!(reqs[0].authority, "api.github.com");
        assert_eq!(
            reqs[0].path_with_query,
            "/repos/o/r/pulls?state=all&per_page=100"
        );
        let has = |k: &str, v: &str| {
            reqs[0]
                .headers
                .iter()
                .any(|(hk, hv)| hk.eq_ignore_ascii_case(k) && hv == v)
        };
        assert!(has("authorization", "Bearer tok"));
        assert!(has("user-agent", "moonlit"));
        assert!(has("accept", "application/vnd.github+json"));
        assert!(has("x-github-api-version", "2022-11-28"));
    }

    #[test]
    fn follows_link_header_to_second_page() {
        let host = MockHost::new()
            .with_http_response_headers(
                200,
                vec![(
                    "link".into(),
                    "<https://api.github.com/x?page=2>; rel=\"next\"".into(),
                )],
                b"[{\"number\":1}]",
            )
            .with_http_response(200, b"[{\"number\":2}]");
        let ctx = Context::new(&host, "/w".into(), "s".into());
        let items = get_paginated(&ctx, "tok", "/x").unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(host.recorded_requests().len(), 2);
        assert_eq!(host.recorded_requests()[1].path_with_query, "/x?page=2");
    }

    #[test]
    fn auth_error_maps_401() {
        let host = MockHost::new().with_http_response(401, b"{}");
        let ctx = Context::new(&host, "/w".into(), "s".into());
        let e = match get_paginated(&ctx, "tok", "/x") {
            Ok(_) => panic!("401 must be an error"),
            Err(e) => e,
        };
        assert!(e.contains("authentication failed"), "got: {e}");
    }

    #[test]
    fn post_sends_json_body() {
        let host = MockHost::new().with_http_response(201, br#"{"html_url":"u"}"#);
        let ctx = Context::new(&host, "/w".into(), "s".into());
        let r = post_json(
            &ctx,
            "tok",
            "/repos/o/r/releases",
            &serde_json::json!({"name":"1.0"}),
        )
        .unwrap();
        assert!(r.is_success());
        let reqs = host.recorded_requests();
        assert_eq!(reqs[0].path_with_query, "/repos/o/r/releases");
        assert_eq!(reqs[0].body.as_deref().unwrap(), br#"{"name":"1.0"}"#);
    }
}
