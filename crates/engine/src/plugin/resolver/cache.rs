use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub trait Clock: Send + Sync {
    fn now_unix(&self) -> u64;
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now_unix(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginMeta {
    pub source: String,
    pub digest: Option<String>,
    pub layer_digest: Option<String>,
    pub size: u64,
    pub pulled_at: u64,
    pub middlewares: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RefRecord {
    digest: String,
    resolved_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanStats {
    pub plugins: usize,
    pub blobs: usize,
    pub refs: usize,
    pub bytes: u64,
}

pub struct Cache {
    root: PathBuf,
    clock: Box<dyn Clock>,
}

impl Cache {
    pub fn new() -> std::io::Result<Self> {
        let base = dirs::cache_dir()
            .ok_or_else(|| std::io::Error::other("could not determine the OS cache directory"))?;
        Ok(Self {
            root: base.join("moonlit"),
            clock: Box::new(SystemClock),
        })
    }

    pub fn with_root_and_clock(root: PathBuf, clock: Box<dyn Clock>) -> Self {
        Self { root, clock }
    }

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

    pub fn blob_path(&self, digest: &str) -> PathBuf {
        let (algo, hex) = digest.split_once(':').unwrap_or(("sha256", digest));
        self.root.join("oci").join(algo).join(hex)
    }

    pub fn write_blob(&self, digest: &str, bytes: &[u8]) -> std::io::Result<()> {
        let path = self.blob_path(digest);
        if path.is_file() {
            return Ok(());
        }
        write_atomic(&path, bytes)
    }

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

    pub fn read_meta(&self, key: &str) -> Option<PluginMeta> {
        let bytes = std::fs::read(self.plugin_dir(key).join("meta.json")).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

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

    pub fn list(&self) -> Vec<(String, PluginMeta)> {
        let mut out = Vec::new();
        let Ok(entries) = std::fs::read_dir(self.root.join("plugins")) else {
            return out;
        };
        for entry in entries.flatten() {
            if !entry.path().is_dir() {
                continue;
            }
            let key = entry.file_name().to_string_lossy().into_owned();
            if let Some(meta) = self.read_meta(&key) {
                out.push((key, meta));
            }
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    pub fn clean(&self) -> std::io::Result<CleanStats> {
        let plugins_dir = self.root.join("plugins");
        let oci_dir = self.root.join("oci");
        let refs_dir = self.root.join("refs");

        let stats = CleanStats {
            plugins: count_immediate_dirs(&plugins_dir),
            blobs: count_files_recursive(&oci_dir),
            refs: count_files_recursive(&refs_dir),
            bytes: dir_size(&plugins_dir) + dir_size(&oci_dir) + dir_size(&refs_dir),
        };
        for dir in [&plugins_dir, &oci_dir, &refs_dir] {
            if dir.exists() {
                std::fs::remove_dir_all(dir)?;
            }
        }
        Ok(stats)
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("cache path has no parent directory"))?;
    std::fs::create_dir_all(parent)?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
    std::io::Write::write_all(&mut tmp, bytes)?;
    tmp.persist(path).map_err(|e| e.error)?;
    Ok(())
}

fn dir_size(dir: &Path) -> u64 {
    let mut total = 0;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                total += dir_size(&p);
            } else if let Ok(m) = p.metadata() {
                total += m.len();
            }
        }
    }
    total
}

fn count_immediate_dirs(dir: &Path) -> usize {
    std::fs::read_dir(dir)
        .map(|it| it.flatten().filter(|e| e.path().is_dir()).count())
        .unwrap_or(0)
}

fn count_files_recursive(dir: &Path) -> usize {
    let mut n = 0;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                n += count_files_recursive(&p);
            } else {
                n += 1;
            }
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Duration;

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

    struct TestClock;
    impl Clock for TestClock {
        fn now_unix(&self) -> u64 {
            0
        }
    }

    #[test]
    fn list_returns_sorted_plugins_and_skips_unreadable() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::with_root_and_clock(dir.path().to_path_buf(), Box::new(TestClock));
        let meta = |src: &str| PluginMeta {
            source: src.to_string(),
            digest: Some("sha256:d".to_string()),
            layer_digest: Some("sha256:l".to_string()),
            size: 3,
            pulled_at: 0,
            middlewares: Some(vec!["build".to_string()]),
        };
        cache
            .store_plugin("sha256-b", &meta("oci://b"), b"bbb")
            .unwrap();
        cache
            .store_plugin("sha256-a", &meta("oci://a"), b"aaa")
            .unwrap();
        std::fs::create_dir_all(cache.plugin_dir("sha256-c")).unwrap();

        let listed = cache.list();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].0, "sha256-a");
        assert_eq!(listed[1].0, "sha256-b");
        assert_eq!(listed[0].1.source, "oci://a");
    }

    #[test]
    fn list_on_empty_cache_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::with_root_and_clock(dir.path().to_path_buf(), Box::new(TestClock));
        assert!(cache.list().is_empty());
    }

    #[test]
    fn clean_removes_all_content_and_reports_stats() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::with_root_and_clock(dir.path().to_path_buf(), Box::new(TestClock));
        let meta = PluginMeta {
            source: "oci://x".to_string(),
            digest: Some("sha256:d".to_string()),
            layer_digest: Some("sha256:l".to_string()),
            size: 3,
            pulled_at: 0,
            middlewares: None,
        };
        cache.store_plugin("sha256-x", &meta, b"xyz").unwrap();
        cache.write_blob("sha256:layer", b"\0asm").unwrap();
        cache.write_ref("reg/x:1", "sha256:d").unwrap();

        let stats = cache.clean().unwrap();
        assert_eq!(stats.plugins, 1);
        assert_eq!(stats.refs, 1);
        assert!(stats.blobs >= 1);
        assert!(stats.bytes > 0);
        assert!(cache.list().is_empty());
        assert!(!dir.path().join("plugins").exists());
        assert!(!dir.path().join("oci").exists());
        assert!(!dir.path().join("refs").exists());
    }

    #[test]
    fn clean_on_empty_cache_is_ok_and_zero() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::with_root_and_clock(dir.path().to_path_buf(), Box::new(TestClock));
        let stats = cache.clean().unwrap();
        assert_eq!(stats.plugins, 0);
        assert_eq!(stats.blobs, 0);
        assert_eq!(stats.refs, 0);
        assert_eq!(stats.bytes, 0);
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
        assert_eq!(std::fs::read(&*path).unwrap(), payload);
        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|x| x == "tmp").unwrap_or(false))
            .collect();
        assert!(leftovers.is_empty(), "leaked temp files: {leftovers:?}");
    }
}
