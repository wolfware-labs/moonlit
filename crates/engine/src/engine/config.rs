use std::path::PathBuf;
use std::time::Duration;

pub struct EngineSettings {
    pub cache_dir: Option<PathBuf>,
    pub tag_ttl: Duration,
}

impl Default for EngineSettings {
    fn default() -> Self {
        Self {
            cache_dir: None,
            tag_ttl: Duration::from_mins(15),
        }
    }
}
