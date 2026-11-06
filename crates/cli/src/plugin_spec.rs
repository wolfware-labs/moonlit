/// A plugin named on the command line: a registry package or a direct URL.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PluginSpec {
  Registry {
    org: String,
    name: String,
    version: Option<String>,
  },
  Url(String),
}

const URL_SCHEMES: [&str; 4] = ["oci://", "file://", "http://", "https://"];

impl PluginSpec {
  /// Parses `org/name`, `org/name@1.2.3`, or an `oci://`, `file://` or `http(s)://` URL.
  pub fn parse(s: &str) -> Result<PluginSpec, String> {
    let s = s.trim();
    if URL_SCHEMES.iter().any(|scheme| s.starts_with(scheme)) {
      return Ok(PluginSpec::Url(s.to_string()));
    }
    if s.contains("://") {
      return Err(format!(
        "'{s}' has an unsupported scheme; expected oci://, file://, http:// or https://"
      ));
    }
    let (package, version) = match s.split_once('@') {
      Some((package, version)) if !version.is_empty() => (package, Some(version.to_string())),
      Some(_) => return Err(format!("'{s}' names no version after '@'")),
      None => (s, None),
    };
    match package.split_once('/') {
      Some((org, name)) if !org.is_empty() && !name.is_empty() && !name.contains('/') => Ok(PluginSpec::Registry {
        org: org.to_string(),
        name: name.to_string(),
        version,
      }),
      _ => Err(format!("'{s}' is not a plugin; expected org/name, org/name@version or a URL")),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn registry(org: &str, name: &str, version: Option<&str>) -> PluginSpec {
    PluginSpec::Registry {
      org: org.into(),
      name: name.into(),
      version: version.map(Into::into),
    }
  }

  #[test]
  fn parses_registry_packages() {
    assert_eq!(PluginSpec::parse("wolfware/github"), Ok(registry("wolfware", "github", None)));
    assert_eq!(
      PluginSpec::parse("wolfware/github@2.0.0"),
      Ok(registry("wolfware", "github", Some("2.0.0")))
    );
  }

  #[test]
  fn parses_urls() {
    for url in [
      "oci://ghcr.io/acme/x:1.0.0",
      "file:///tmp/x.wasm",
      "http://example.com/x.wasm",
      "https://example.com/x.wasm",
    ] {
      assert_eq!(PluginSpec::parse(url), Ok(PluginSpec::Url(url.into())));
    }
  }

  #[test]
  fn rejects_malformed_specs() {
    for spec in ["github", "/github", "wolfware/", "a/b/c", "wolfware/github@", "ftp://x/y"] {
      assert!(PluginSpec::parse(spec).is_err(), "{spec}");
    }
  }
}
