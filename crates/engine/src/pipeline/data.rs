use crate::expr::{Resolve, Value};
use crate::pipeline::config::PipelineConfig;
use indexmap::IndexMap;

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
  pub fn new(cfg: PipelineConfig) -> Self {
    let env: Vec<(String, String)> = std::env::vars().collect();
    let dotenv = std::fs::read_to_string(opts.working_directory.join(".env")).ok();
    let base = Self::build_base_layer(&env, dotenv.as_deref());
    let release = Self::build_release_layer(&cfg.variables, &cfg.arguments, &opts.cli_args);

    let mut pipeline_data = Self { layers: Vec::new() };
    pipeline_data.push(base.clone());
    pipeline_data.push(release.clone());
    pipeline_data
  }

  pub fn push(&mut self, layer: Value) {
    self.layers.push(layer);
  }

  pub fn resolve(&self, path: &str) -> Option<Value> {
    for layer in self.layers.iter().rev() {
      if let Some(v) = crate::expr::accumulator::lookup(layer, path) {
        return Some(v);
      }
    }
    None
  }

  pub fn merged(&self, section: &str) -> Value {
    let mut out: IndexMap<String, Value> = IndexMap::new();
    for layer in &self.layers {
      if let crate::expr::Value::Map(m) = layer
        && let Some(crate::expr::Value::Map(sec)) = m.get(section)
      {
        crate::expr::accumulator::deep_merge(&mut out, sec);
      }
    }
    crate::expr::Value::Map(out)
  }

  pub fn build_base_layer(env: &[(String, String)], dotenv: Option<&str>) -> Value {
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

  pub fn build_release_layer(
    vars: &IndexMap<String, String>,
    args: &IndexMap<String, String>,
    cli_args: &[(String, String)],
  ) -> Value {
    let to_map =
      |src: &IndexMap<String, String>| Value::Map(src.iter().map(|(k, v)| (k.clone(), Value::Str(v.clone()))).collect());
    let mut args_map = match to_map(args) {
      Value::Map(m) => m,
      _ => unreachable!(),
    };
    for (k, v) in cli_args {
      args_map.insert(k.clone(), Value::Str(v.clone()));
    }
    let mut root = IndexMap::new();
    root.insert("vars".to_string(), to_map(vars));
    root.insert("args".to_string(), Value::Map(args_map));
    Value::Map(root)
  }
}

fn lookup(root: &crate::expr::Value, path: &str) -> Option<crate::expr::Value> {
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
      (Some(Value::Map(d)), Value::Map(s)) => deep_merge(d, s),
      _ => {
        dst.insert(k.clone(), v.clone());
      }
    }
  }
}
