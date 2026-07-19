//! Content-addressed on-disk cache for resolved plugins (§7.3, §8.3).
//!
//! Layout under `<cache-root>/moonlit/`:
//! ```text
//! oci/sha256/<hex>       # OCI layer blobs, content-addressed by digest
//! plugins/<key>/         # a resolved plugin: plugin.wasm + meta.json
//! refs/<hash>.json       # OCI tag -> digest resolution cache with a timestamp
//! ```
//! OCI keys `plugins/` by the manifest digest; `http` keys by `sha256(url)`; `file` is not cached.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::resolve::sha256_hex;

/// A clock, injectable so TTL logic is testable without sleeping.
pub trait Clock: Send + Sync {
    /// Seconds since the Unix epoch.
    fn now_unix(&self) -> u64;
}

/// The real wall-clock.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_unix(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}

/// Metadata persisted alongside a cached plugin (`meta.json`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginMeta {
    pub source: String,
    pub digest: Option<String>,
    pub layer_digest: Option<String>,
    pub size: u64,
    pub pulled_at: u64,
    pub middlewares: Option<Vec<String>>,
}

/// The tag→digest resolution record stored in `refs/`.
#[derive(Debug, Serialize, Deserialize)]
struct RefRecord {
    digest: String,
    resolved_at: u64,
}

/// The plugin content cache.
pub struct Cache {
    root: PathBuf,
    clock: Box<dyn Clock>,
}

impl Cache {
    /// Open the default cache (`<OS cache dir>/moonlit`) with the system clock.
    pub fn new() -> std::io::Result<Self> {
        let base = dirs::cache_dir()
            .ok_or_else(|| std::io::Error::other("could not determine the OS cache directory"))?;
        Ok(Self {
            root: base.join("moonlit"),
            clock: Box::new(SystemClock),
        })
    }

    /// Construct a cache rooted at `root` with an explicit clock (used in tests).
    pub fn with_root_and_clock(root: PathBuf, clock: Box<dyn Clock>) -> Self {
        Self { root, clock }
    }

    /// Seconds since the Unix epoch, per this cache's clock.
    pub fn now_unix(&self) -> u64 {
        self.clock.now_unix()
    }

    pub fn plugin_dir(&self, key: &str) -> PathBuf {
        self.root.join("plugins").join(key)
    }

    pub fn plugin_wasm(&self, key: &str) -> PathBuf {
        self.plugin_dir(key).join("plugin.wasm")
    }

    pub fn has_plugin(&self, key: &str) -> bool {
        self.plugin_wasm(key).is_file()
    }

    /// Path to a content-addressed blob. `digest` is `algo:hex` (e.g. `sha256:abcd`).
    pub fn blob_path(&self, digest: &str) -> PathBuf {
        let (algo, hex) = digest.split_once(':').unwrap_or(("sha256", digest));
        self.root.join("oci").join(algo).join(hex)
    }

    /// Write a content-addressed blob (idempotent — skips if already present).
    pub fn write_blob(&self, digest: &str, bytes: &[u8]) -> std::io::Result<()> {
        let path = self.blob_path(digest);
        if path.is_file() {
            return Ok(());
        }
        write_atomic(&path, bytes)
    }

    /// Store a resolved plugin's bytes + metadata under `plugins/<key>/`. Returns the wasm path.
    pub fn store_plugin(
        &self,
        key: &str,
        meta: &PluginMeta,
        bytes: &[u8],
    ) -> std::io::Result<PathBuf> {
        let wasm = self.plugin_wasm(key);
        write_atomic(&wasm, bytes)?;
        let meta_json = serde_json::to_vec_pretty(meta).map_err(std::io::Error::other)?;
        write_atomic(&self.plugin_dir(key).join("meta.json"), &meta_json)?;
        Ok(wasm)
    }

    /// Read a cached plugin's metadata, if present and parseable.
    pub fn read_meta(&self, key: &str) -> Option<PluginMeta> {
        let bytes = std::fs::read(self.plugin_dir(key).join("meta.json")).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    /// Return a cached tag→digest resolution if it is still within `ttl`.
    pub fn read_ref(&self, oci_ref: &str, ttl: Duration) -> Option<String> {
        let bytes = std::fs::read(self.ref_path(oci_ref)).ok()?;
        let record: RefRecord = serde_json::from_slice(&bytes).ok()?;
        let age = self.now_unix().saturating_sub(record.resolved_at);
        if age <= ttl.as_secs() {
            Some(record.digest)
        } else {
            None
        }
    }

    /// Record a tag→digest resolution stamped with the current time.
    pub fn write_ref(&self, oci_ref: &str, digest: &str) -> std::io::Result<()> {
        let record = RefRecord {
            digest: digest.to_string(),
            resolved_at: self.now_unix(),
        };
        let bytes = serde_json::to_vec(&record).map_err(std::io::Error::other)?;
        write_atomic(&self.ref_path(oci_ref), &bytes)
    }

    fn ref_path(&self, oci_ref: &str) -> PathBuf {
        self.root
            .join("refs")
            .join(format!("{}.json", sha256_hex(oci_ref)))
    }
}

/// Write `bytes` to `path`, creating parent directories. Writes to a temp sibling then renames so a
/// crash mid-write never leaves a partial file at `path`.
fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("cache path has no parent directory"))?;
    std::fs::create_dir_all(parent)?;
    // A uniquely-named temp in the SAME directory (O_EXCL), so the rename is atomic on one
    // filesystem and two concurrent writers to the same key never share a temp path.
    let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
    std::io::Write::write_all(&mut tmp, bytes)?;
    tmp.persist(path).map_err(|e| e.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Duration;

    /// A controllable clock for TTL tests.
    struct MockClock(Arc<AtomicU64>);
    impl Clock for MockClock {
        fn now_unix(&self) -> u64 {
            self.0.load(Ordering::SeqCst)
        }
    }

    fn cache_with_clock(now: Arc<AtomicU64>) -> (Cache, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::with_root_and_clock(dir.path().to_path_buf(), Box::new(MockClock(now)));
        (cache, dir)
    }

    #[test]
    fn plugin_paths_are_under_plugins_key() {
        let (cache, _d) = cache_with_clock(Arc::new(AtomicU64::new(0)));
        assert!(cache.plugin_dir("abc").ends_with("plugins/abc"));
        assert!(
            cache
                .plugin_wasm("abc")
                .ends_with("plugins/abc/plugin.wasm")
        );
    }

    #[test]
    fn blob_path_splits_digest_algo_and_hex() {
        let (cache, _d) = cache_with_clock(Arc::new(AtomicU64::new(0)));
        assert!(
            cache
                .blob_path("sha256:deadbeef")
                .ends_with("oci/sha256/deadbeef")
        );
    }

    #[test]
    fn store_and_read_plugin_round_trips_bytes_and_meta() {
        let (cache, _d) = cache_with_clock(Arc::new(AtomicU64::new(1000)));
        let meta = PluginMeta {
            source: "oci://reg/x:1".to_string(),
            digest: Some("sha256:m".to_string()),
            layer_digest: Some("sha256:l".to_string()),
            size: 3,
            pulled_at: 1000,
            middlewares: Some(vec!["build".to_string()]),
        };
        assert!(!cache.has_plugin("k"));
        let path = cache.store_plugin("k", &meta, &[1, 2, 3]).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), vec![1, 2, 3]);
        assert!(cache.has_plugin("k"));
        let read = cache.read_meta("k").unwrap();
        assert_eq!(read.digest.as_deref(), Some("sha256:m"));
        assert_eq!(read.middlewares, Some(vec!["build".to_string()]));
    }

    #[test]
    fn write_blob_writes_content_addressed_file() {
        let (cache, _d) = cache_with_clock(Arc::new(AtomicU64::new(0)));
        cache.write_blob("sha256:abcd", &[9, 9]).unwrap();
        assert_eq!(
            std::fs::read(cache.blob_path("sha256:abcd")).unwrap(),
            vec![9, 9]
        );
    }

    #[test]
    fn read_ref_returns_digest_within_ttl() {
        let now = Arc::new(AtomicU64::new(1_000));
        let (cache, _d) = cache_with_clock(now.clone());
        cache.write_ref("reg/x:tag", "sha256:m").unwrap();
        // 10 minutes later, within a 15-minute TTL
        now.store(1_000 + 600, Ordering::SeqCst);
        assert_eq!(
            cache
                .read_ref("reg/x:tag", Duration::from_secs(900))
                .as_deref(),
            Some("sha256:m")
        );
    }

    #[test]
    fn read_ref_expires_past_ttl() {
        let now = Arc::new(AtomicU64::new(1_000));
        let (cache, _d) = cache_with_clock(now.clone());
        cache.write_ref("reg/x:tag", "sha256:m").unwrap();
        // 20 minutes later, past a 15-minute TTL
        now.store(1_000 + 1_200, Ordering::SeqCst);
        assert_eq!(cache.read_ref("reg/x:tag", Duration::from_secs(900)), None);
    }

    #[test]
    fn read_ref_missing_is_none() {
        let (cache, _d) = cache_with_clock(Arc::new(AtomicU64::new(0)));
        assert_eq!(
            cache.read_ref("reg/never:tag", Duration::from_secs(900)),
            None
        );
    }

    #[test]
    fn write_atomic_is_concurrency_safe_for_one_key() {
        use std::sync::Arc;
        let dir = tempfile::tempdir().unwrap();
        let path = Arc::new(dir.path().join("blob.bin"));
        let payload = vec![7u8; 4096];
        let mut handles = Vec::new();
        for _ in 0..16 {
            let p = Arc::clone(&path);
            let bytes = payload.clone();
            handles.push(std::thread::spawn(move || {
                super::write_atomic(&p, &bytes).unwrap()
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        // Final file is intact (exactly the payload), and no stray *.tmp sibling leaked.
        assert_eq!(std::fs::read(&*path).unwrap(), payload);
        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|x| x == "tmp").unwrap_or(false))
            .collect();
        assert!(leftovers.is_empty(), "leaked temp files: {leftovers:?}");
    }
}
