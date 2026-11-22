//! Redirect rules shared by the engine's HTTP clients (the plugin download and the registry).

/// Redirects a request follows before it gives up, as many as reqwest's own default.
pub(crate) const MAX_REDIRECTS: usize = 10;

/// Whether a request that already went to `previous` (the first URL, then each hop) has used up its redirects.
pub(crate) fn too_many(previous: &[reqwest::Url]) -> bool {
  previous.len() > MAX_REDIRECTS
}

/// Whether a redirect to `next` takes a request that was on https, at its last hop in `previous`, to another scheme.
pub(crate) fn leaves_https(previous: &[reqwest::Url], next: &reqwest::Url) -> bool {
  previous.last().is_some_and(|p| p.scheme() == "https") && next.scheme() != "https"
}

/// The refusal of type `E` that a redirect policy raised behind a failed request, found down `error`'s sources.
pub(crate) fn refusal<E: std::error::Error + 'static>(error: &reqwest::Error) -> Option<&E> {
  let mut source = std::error::Error::source(error);
  while let Some(cause) = source {
    if let Some(refusal) = cause.downcast_ref::<E>() {
      return Some(refusal);
    }
    source = cause.source();
  }
  None
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn too_many_allows_max_redirects_and_stops_the_next() {
    let hops: Vec<reqwest::Url> = (0..=MAX_REDIRECTS)
      .map(|n| reqwest::Url::parse(&format!("https://h{n}.example.com/p")).unwrap())
      .collect();
    assert!(!too_many(&hops[..MAX_REDIRECTS]));
    assert!(too_many(&hops));
  }

  #[test]
  fn leaves_https_looks_at_the_hop_before_the_redirect() {
    let url = |u: &str| reqwest::Url::parse(u).unwrap();
    assert!(leaves_https(
      &[url("https://a.example.com/p")],
      &url("http://b.example.com/p")
    ));
    assert!(leaves_https(
      &[url("http://a.example.com/p"), url("https://b.example.com/p")],
      &url("http://127.0.0.1/p")
    ));
    assert!(!leaves_https(
      &[url("https://a.example.com/p")],
      &url("https://b.example.com/p")
    ));
    assert!(!leaves_https(
      &[url("http://a.example.com/p")],
      &url("http://b.example.com/p")
    ));
    assert!(!leaves_https(&[], &url("http://b.example.com/p")));
  }
}
