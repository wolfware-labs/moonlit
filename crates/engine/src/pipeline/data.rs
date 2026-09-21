use indexmap::IndexMap;

#[derive(Debug, Default)]
pub struct PipelineData {
    layers: Vec<crate::expr::value::Value>,
}

impl Resolve for PipelineData {
    fn resolve(&self, path: &str) -> Option<Value> {
        PipelineData::resolve(self, path)
    }
}

impl PipelineData {
    pub fn new() -> Self {
        let env: Vec<(String, String)> = std::env::vars().collect();
        let dotenv = std::fs::read_to_string(opts.working_directory.join(".env")).ok();
        let base = Self::build_base_layer(&env, dotenv.as_deref());
        let release = Self::build_release_layer(&cfg.variables, &cfg.arguments, &opts.cli_args);

        let mut subst_acc = Self { layers: Vec::new() };
        subst_acc.push(base.clone());
        subst_acc.push(release.clone());
    }

    pub fn push(&mut self, layer: crate::expr::value::Value) {
        self.layers.push(layer);
    }

    pub fn resolve(&self, path: &str) -> Option<crate::expr::value::Value> {
        for layer in self.layers.iter().rev() {
            if let Some(v) = crate::expr::accumulator::lookup(layer, path) {
                return Some(v);
            }
        }
        None
    }

    pub fn merged(&self, section: &str) -> crate::expr::value::Value {
        let mut out: IndexMap<String, crate::expr::value::Value> = IndexMap::new();
        for layer in &self.layers {
            if let crate::expr::value::Value::Map(m) = layer
                && let Some(crate::expr::value::Value::Map(sec)) = m.get(section)
            {
                crate::expr::accumulator::deep_merge(&mut out, sec);
            }
        }
        crate::expr::value::Value::Map(out)
    }

    pub fn build_base_layer(
        env: &[(String, String)],
        dotenv: Option<&str>,
    ) -> crate::expr::value::Value {
        let mut map: IndexMap<String, crate::expr::value::Value> = IndexMap::new();
        if let Some(contents) = dotenv {
            for (k, v) in dotenvy::from_read_iter(contents.as_bytes()).flatten() {
                map.insert(k, crate::expr::value::Value::Str(v));
            }
        }
        for (k, v) in env {
            if let Some(stripped) = k.strip_prefix("MOONLIT_") {
                map.insert(
                    stripped.to_string(),
                    crate::expr::value::Value::Str(v.clone()),
                );
            }
        }
        crate::expr::value::Value::Map(map)
    }

    pub fn build_release_layer(
        vars: &IndexMap<String, String>,
        args: &IndexMap<String, String>,
        cli_args: &[(String, String)],
    ) -> crate::expr::value::Value {
        let to_map = |src: &IndexMap<String, String>| {
            crate::expr::value::Value::Map(
                src.iter()
                    .map(|(k, v)| (k.clone(), crate::expr::value::Value::Str(v.clone())))
                    .collect(),
            )
        };
        let mut args_map = match to_map(args) {
            crate::expr::value::Value::Map(m) => m,
            _ => unreachable!(),
        };
        for (k, v) in cli_args {
            args_map.insert(k.clone(), crate::expr::value::Value::Str(v.clone()));
        }
        let mut root = IndexMap::new();
        root.insert("vars".to_string(), to_map(vars));
        root.insert("args".to_string(), crate::expr::value::Value::Map(args_map));
        crate::expr::value::Value::Map(root)
    }
}

fn lookup(root: &crate::expr::value::Value, path: &str) -> Option<crate::expr::value::Value> {
    let mut cur = root;
    for seg in path.split(':') {
        match cur {
            crate::expr::value::Value::Map(m) => cur = m.get(seg)?,
            crate::expr::value::Value::List(l) => {
                let i: usize = seg.parse().ok()?;
                cur = l.get(i)?;
            }
            _ => return None,
        }
    }
    match cur {
        crate::expr::value::Value::Null => None,
        other => Some(other.clone()),
    }
}

fn deep_merge(
    dst: &mut IndexMap<String, crate::expr::value::Value>,
    src: &IndexMap<String, crate::expr::value::Value>,
) {
    for (k, v) in src {
        match (dst.get_mut(k), v) {
            (Some(crate::expr::value::Value::Map(d)), crate::expr::value::Value::Map(s)) => {
                deep_merge(d, s)
            }
            _ => {
                dst.insert(k.clone(), v.clone());
            }
        }
    }
}
