pub mod login;
pub mod logout;

use std::path::Path;
use std::time::Duration;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

#[must_use]
fn read_bearer(home: &Path, host: &str) -> Option<String> {
  let path = home.join(".config/moonlit/credentials.toml");
  let doc: toml::Table = std::fs::read_to_string(&path).ok()?.parse().ok()?;
  doc
    .get("registries")?
    .as_table()?
    .get(host)?
    .as_table()?
    .get("token")?
    .as_str()
    .map(str::to_string)
}

fn http_client(timeout: Duration) -> reqwest::Result<reqwest::Client> {
  reqwest::Client::builder()
    .timeout(timeout)
    .connect_timeout(CONNECT_TIMEOUT.min(timeout))
    .build()
}

#[must_use]
fn base_url(host: &str) -> String {
  let scheme = if is_loopback(host) { "http" } else { "https" };
  format!("{scheme}://{host}")
}

#[must_use]
fn is_loopback(host: &str) -> bool {
  let hostname = if let Some(rest) = host.strip_prefix('[') {
    rest.split(']').next().unwrap_or(rest)
  } else {
    match host.rsplit_once(':') {
      Some((h, port)) if !h.contains(':') && !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()) => h,
      _ => host,
    }
  };
  hostname.eq_ignore_ascii_case("localhost") || hostname == "127.0.0.1" || hostname == "::1"
}

fn write_doc_0600(path: &Path, text: &str) -> std::io::Result<()> {
  if let Some(parent) = path.parent() {
    std::fs::create_dir_all(parent)?;
  }

  #[cfg(unix)]
  {
    use std::io::Write as _;
    use std::os::unix::fs::OpenOptionsExt;
    let mut tmp_os = path.as_os_str().to_os_string();
    tmp_os.push(".tmp");
    let tmp = std::path::PathBuf::from(tmp_os);
    let mut f = std::fs::OpenOptions::new()
      .write(true)
      .create(true)
      .truncate(true)
      .mode(0o600)
      .open(&tmp)?;
    f.write_all(text.as_bytes())?;
    f.sync_all()?;
    std::fs::rename(&tmp, path)?;
  }
  #[cfg(not(unix))]
  {
    std::fs::write(path, text)?;
  }
  Ok(())
}

#[must_use]
fn home_dir() -> Option<std::path::PathBuf> {
  moonlit_engine::paths::home_dir()
}

#[cfg(test)]
mod tests {
  use super::*;

  fn write_credentials(home: &Path, text: &str) {
    let path = home.join(".config/moonlit/credentials.toml");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
  }

  #[test]
  fn loopback_hosts_use_plain_http() {
    for host in [
      "localhost",
      "LOCALHOST:5185",
      "127.0.0.1",
      "127.0.0.1:80",
      "[::1]",
      "[::1]:5185",
      "::1",
    ] {
      assert!(is_loopback(host), "{host} should be loopback");
      assert!(base_url(host).starts_with("http://"), "{host}");
    }
  }

  #[test]
  fn remote_hosts_use_https() {
    for host in [
      "registry.moonlit.rs",
      "example.com:443",
      "10.0.0.1",
      "localhost.evil.com",
      "host:port",
    ] {
      assert!(!is_loopback(host), "{host} should not be loopback");
    }
    assert_eq!(base_url("registry.moonlit.rs"), "https://registry.moonlit.rs");
  }

  #[test]
  fn bearer_is_read_for_the_matching_host_only() {
    let home = tempfile::tempdir().unwrap();
    write_credentials(
      home.path(),
      "[registries.\"a.io\"]\ntoken = \"t-a\"\n[registries.\"b.io\"]\nusername = \"u\"\npassword = \"p\"\n",
    );
    assert_eq!(read_bearer(home.path(), "a.io"), Some("t-a".to_string()));
    assert_eq!(read_bearer(home.path(), "b.io"), None);
    assert_eq!(read_bearer(home.path(), "c.io"), None);
  }

  #[test]
  fn bearer_is_none_without_a_readable_credentials_file() {
    let home = tempfile::tempdir().unwrap();
    assert_eq!(read_bearer(home.path(), "a.io"), None);
    write_credentials(home.path(), "not = [valid");
    assert_eq!(read_bearer(home.path(), "a.io"), None);
  }

  #[test]
  fn credentials_file_is_written_with_its_parent_directories() {
    let home = tempfile::tempdir().unwrap();
    let path = home.path().join("nested/dir/credentials.toml");
    write_doc_0600(&path, "x = 1\n").unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "x = 1\n");
  }

  #[cfg(unix)]
  #[test]
  fn credentials_file_is_private_on_unix() {
    use std::os::unix::fs::PermissionsExt;
    let home = tempfile::tempdir().unwrap();
    let path = home.path().join("credentials.toml");
    write_doc_0600(&path, "x = 1\n").unwrap();
    assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
  }

  #[test]
  fn http_client_builds_with_a_short_timeout() {
    assert!(http_client(Duration::from_secs(1)).is_ok());
  }
}
