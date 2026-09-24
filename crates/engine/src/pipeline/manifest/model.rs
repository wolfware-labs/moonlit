use serde::Deserialize;

#[derive(Deserialize)]
pub struct ManifestPeek {
    pub name: Option<String>,
    pub stages: Option<indexmap::IndexMap<String, serde::de::IgnoredAny>>,
}
