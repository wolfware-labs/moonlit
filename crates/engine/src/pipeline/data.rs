use crate::pipeline::config::PipelineConfig;
use crate::pipeline::expr::{Resolve, Value};
use indexmap::IndexMap;
use std::path::Path;

#[derive(Debug, Default)]
pub struct PipelineData {
  layers: Vec<Value>,
}

impl Resolve for PipelineData {
  fn resolve(&self, path: &str) -> Option<Value> {
    PipelineData::resolve(self, path)
  }
}

impl PipelineData {
  #[must_use]
  pub fn new(config: &PipelineConfig, working_dir: &Path) -> Self {
    let env_layer = Self::build_env_layer(working_dir);
    let release_layer = Self::build_release_layer(&config.variables, &config.arguments);

    Self {
      layers: vec![env_layer, release_layer],
    }
  }

  pub fn push(&mut self, layer: Value) {
    self.layers.push(layer);
  }

  #[must_use]
  pub fn resolve(&self, path: &str) -> Option<Value> {
    for layer in self.layers.iter().rev() {
      if let Some(v) = Self::lookup(layer, path) {
        return Some(v);
      }
    }
    None
  }

  #[must_use]
  pub fn merged(&self, section: &str) -> Value {
    let mut out: IndexMap<String, Value> = IndexMap::new();
    for layer in &self.layers {
      if let Value::Map(m) = layer
        && let Some(Value::Map(sec)) = m.get(section)
      {
        Self::deep_merge(&mut out, sec);
      }
    }
    Value::Map(out)
  }

  #[must_use]
  pub fn build_env_layer(working_directory: &Path) -> Value {
    let dotenv = std::fs::read_to_string(working_directory.join(".env")).ok();
    Self::env_layer(std::env::vars(), dotenv.as_deref())
  }

  #[must_use]
  fn env_layer(env: impl IntoIterator<Item = (String, String)>, dotenv: Option<&str>) -> Value {
    let mut map: IndexMap<String, Value> = IndexMap::new();
    if let Some(contents) = dotenv {
      for (k, v) in dotenvy::from_read_iter(contents.as_bytes()).flatten() {
        map.insert(k, Value::Str(v));
      }
    }
    for (k, v) in env {
      if let Some(stripped) = k.strip_prefix("MOONLIT_") {
        map.insert(stripped.to_string(), Value::Str(v));
      }
    }
    Value::Map(map)
  }

  #[must_use]
  pub fn build_release_layer(vars: &IndexMap<String, String>, args: &IndexMap<String, String>) -> Value {
    let to_map =
      |src: &IndexMap<String, String>| Value::Map(src.iter().map(|(k, v)| (k.clone(), Value::Str(v.clone()))).collect());
    let mut root = IndexMap::new();
    root.insert("vars".to_string(), to_map(vars));
    root.insert("args".to_string(), to_map(args));
    Value::Map(root)
  }

  #[must_use]
  fn lookup(root: &Value, path: &str) -> Option<Value> {
    let mut cur = root;
    for seg in path.split(':') {
      match cur {
        Value::Map(m) => cur = m.get(seg)?,
        Value::List(l) => {
          let i: usize = seg.parse().ok()?;
          cur = l.get(i)?;
        }
        _ => return None,
      }
    }
    match cur {
      Value::Null => None,
      other => Some(other.clone()),
    }
  }

  fn deep_merge(dst: &mut IndexMap<String, Value>, src: &IndexMap<String, Value>) {
    for (k, v) in src {
      match (dst.get_mut(k), v) {
        (Some(Value::Map(d)), Value::Map(s)) => Self::deep_merge(d, s),
        _ => {
          dst.insert(k.clone(), v.clone());
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::pipeline::config::parse_config;

  fn map(entries: &[(&str, Value)]) -> Value {
    Value::Map(entries.iter().map(|(k, v)| ((*k).to_string(), v.clone())).collect())
  }

  fn text(s: &str) -> Value {
    Value::Str(s.to_string())
  }

  fn config() -> PipelineConfig {
    let yaml = "arguments:\n  version: '1.0'\nvariables:\n  region: eu\nplugins:\n  - name: tp\n    url: file://tp.wasm\nstages:\n  b:\n    - run: tp.go\n";
    parse_config(yaml, "release.yml").expect("valid configuration")
  }

  #[test]
  fn new_exposes_variables_arguments_and_dotenv() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(".env"), "TOKEN=from-dotenv\n").unwrap();
    let data = PipelineData::new(&config(), dir.path());

    assert_eq!(data.resolve("vars:region"), Some(text("eu")));
    assert_eq!(data.resolve("args:version"), Some(text("1.0")));
    assert_eq!(data.resolve("TOKEN"), Some(text("from-dotenv")));
  }

  #[test]
  fn new_without_dotenv_still_builds() {
    let dir = tempfile::tempdir().unwrap();
    let data = PipelineData::new(&config(), dir.path());
    assert_eq!(data.resolve("args:version"), Some(text("1.0")));
  }

  #[test]
  fn env_layer_keeps_only_moonlit_variables_without_the_prefix() {
    let env = vec![
      ("MOONLIT_TOKEN".to_string(), "abc".to_string()),
      ("PATH".to_string(), "/bin".to_string()),
    ];
    let layer = PipelineData::env_layer(env, None);
    assert_eq!(layer, map(&[("TOKEN", text("abc"))]));
  }

  #[test]
  fn env_layer_prefers_the_environment_over_dotenv() {
    let env = vec![("MOONLIT_TOKEN".to_string(), "env".to_string())];
    let layer = PipelineData::env_layer(env, Some("TOKEN=dotenv\nOTHER=kept\n"));
    assert_eq!(layer, map(&[("TOKEN", text("env")), ("OTHER", text("kept"))]));
  }

  #[test]
  fn later_layers_win() {
    let mut data = PipelineData::default();
    data.push(map(&[("key", text("first"))]));
    data.push(map(&[("key", text("second"))]));
    assert_eq!(data.resolve("key"), Some(text("second")));
  }

  #[test]
  fn null_values_do_not_shadow_earlier_layers() {
    let mut data = PipelineData::default();
    data.push(map(&[("key", text("kept"))]));
    data.push(map(&[("key", Value::Null)]));
    assert_eq!(data.resolve("key"), Some(text("kept")));
  }

  #[test]
  fn resolves_nested_paths_and_list_indexes() {
    let mut data = PipelineData::default();
    data.push(map(&[(
      "output",
      map(&[("s1", map(&[("items", Value::List(vec![text("a"), text("b")]))]))]),
    )]));

    assert_eq!(data.resolve("output:s1:items:1"), Some(text("b")));
    assert_eq!(data.resolve("output:s1:items:9"), None);
    assert_eq!(data.resolve("output:s1:items:x"), None);
    assert_eq!(data.resolve("output:s1:items:0:deeper"), None);
    assert_eq!(data.resolve("output:missing"), None);
  }

  #[test]
  fn resolve_through_the_trait_matches_the_method() {
    let mut data = PipelineData::default();
    data.push(map(&[("key", text("v"))]));
    let resolver: &dyn Resolve = &data;
    assert_eq!(resolver.resolve("key"), Some(text("v")));
  }

  #[test]
  fn merged_deep_merges_a_section_across_layers() {
    let mut data = PipelineData::default();
    data.push(text("not a map"));
    data.push(map(&[("output", map(&[("s1", map(&[("a", text("1"))]))]))]));
    data.push(map(&[("other", map(&[]))]));
    data.push(map(&[(
      "output",
      map(&[("s1", map(&[("b", text("2"))])), ("s2", text("x"))]),
    )]));
    data.push(map(&[("output", map(&[("s2", text("y"))]))]));

    assert_eq!(
      data.merged("output"),
      map(&[("s1", map(&[("a", text("1")), ("b", text("2"))])), ("s2", text("y")),])
    );
  }

  #[test]
  fn merged_ignores_non_map_sections() {
    let mut data = PipelineData::default();
    data.push(map(&[("output", text("scalar"))]));
    assert_eq!(data.merged("output"), map(&[]));
  }
}
