/// The rest of `url` after `scheme` (such as `"oci://"`), matched in any case; `None` when it has another scheme.
#[must_use]
pub fn strip_scheme<'a>(url: &'a str, scheme: &str) -> Option<&'a str> {
  url
    .get(..scheme.len())
    .filter(|head| head.eq_ignore_ascii_case(scheme))
    .map(|_| &url[scheme.len()..])
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn strip_scheme_matches_any_case_and_needs_the_whole_scheme() {
    assert_eq!(strip_scheme("Https://h/x", "https://"), Some("h/x"));
    assert_eq!(strip_scheme("OCI://h/o/n:1", "oci://"), Some("h/o/n:1"));
    assert_eq!(strip_scheme("http", "https://"), None);
    assert_eq!(strip_scheme("https:/h", "https://"), None);
    assert_eq!(strip_scheme("é://x", "oci://"), None);
  }
}
