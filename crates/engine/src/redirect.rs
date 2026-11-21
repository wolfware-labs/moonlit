//! Redirect rules shared by the engine's HTTP clients (the plugin download and the registry).

/// Redirects a request follows before it gives up, as many as reqwest's own default.
pub(crate) const MAX_REDIRECTS: usize = 10;

/// Whether a request that already went to `previous` (the first URL, then each hop) has used up its redirects.
pub(crate) fn too_many(previous: &[reqwest::Url]) -> bool {
  previous.len() > MAX_REDIRECTS
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
}
