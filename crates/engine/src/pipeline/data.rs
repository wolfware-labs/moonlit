use crate::pipeline::config::PipelineConfig;
use crate::pipeline::expr::{Resolve, Value};
use indexmap::IndexMap;
use std::path::PathBuf;

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
  pub fn new(config: PipelineConfig) -> Self {
    let env_layer = Self::build_env_layer(&config.current_dir);
    let release_layer = Self::build_release_layer(&config.variables, &config.arguments);

    Self {
      layers: vec![env_layer, release_layer],
    }
  }

  pub fn push(&mut self, layer: Value) {
    self.layers.push(layer);
  }

  pub fn resolve(&self, path: &str) -> Option<Value> {
    for layer in self.layers.iter().rev() {
      if let Some(v) = Self::lookup(layer, path) {
        return Some(v);
      }
    }
    None
  }

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

  pub fn build_env_layer(working_directory: &PathBuf) -> Value {
    let env: Vec<(String, String)> = std::env::vars().collect();
    let dotenv = std::fs::read_to_string(working_directory.join(".env")).ok();
    let mut map: IndexMap<String, Value> = IndexMap::new();
    if let Some(contents) = dotenv {
      for (k, v) in dotenvy::from_read_iter(contents.as_bytes()).flatten() {
        map.insert(k, Value::Str(v));
      }
    }
    for (k, v) in env {
      if let Some(stripped) = k.strip_prefix("MOONLIT_") {
        map.insert(stripped.to_string(), Value::Str(v.clone()));
      }
    }
    Value::Map(map)
  }

  pub fn build_release_layer(vars: &IndexMap<String, String>, args: &IndexMap<String, String>) -> Value {
    let to_map =
      |src: &IndexMap<String, String>| Value::Map(src.iter().map(|(k, v)| (k.clone(), Value::Str(v.clone()))).collect());
    let mut root = IndexMap::new();
    root.insert("vars".to_string(), to_map(vars));
    root.insert("args".to_string(), to_map(args));
    Value::Map(root)
  }

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
