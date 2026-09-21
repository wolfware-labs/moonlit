pub mod login;
pub mod logout;

use std::path::Path;
use std::time::Duration;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

fn read_bearer(home: &Path, host: &str) -> Option<String> {
    let path = home.join(".config/moonlit/credentials.toml");
    let doc: toml::Table = std::fs::read_to_string(&path).ok()?.parse().ok()?;
    doc.get("registries")?
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

fn base_url(host: &str) -> String {
    let scheme = if is_loopback(host) { "http" } else { "https" };
    format!("{scheme}://{host}")
}

fn is_loopback(host: &str) -> bool {
    let hostname = if let Some(rest) = host.strip_prefix('[') {
        rest.split(']').next().unwrap_or(rest)
    } else {
        match host.rsplit_once(':') {
            Some((h, port))
                if !h.contains(':')
                    && !port.is_empty()
                    && port.bytes().all(|b| b.is_ascii_digit()) =>
            {
                h
            }
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

fn home_dir() -> Option<std::path::PathBuf> {
    dirs::home_dir().filter(|p| !p.as_os_str().is_empty())
}
