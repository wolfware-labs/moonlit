mod config;
mod data;
mod model;

use crate::engine::Engine;
use crate::engine::error::EngineError;
use crate::pipeline::config::PipelineConfig;
use crate::pipeline::data::PipelineData;
use crate::pipeline::model::FlatStep;
pub use crate::pipeline::model::{
    MiddlewareResult, PipelineEvent, PipelineOptions, PipelineSummary, StepResult,
};
use crate::plugin::Plugin;
use indexmap::IndexMap;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::task::JoinSet;

pub struct Pipeline {
    plugins: IndexMap<String, Plugin>,
    steps: Vec<FlatStep>,
    working_directory: PathBuf,
    step_timeout: Option<Duration>,
}

impl Pipeline {
    pub async fn load(
        engine: &Engine,
        config: &PipelineConfig,
        opts: PipelineOptions,
        events: &Sender<PipelineEvent>,
    ) -> Result<Self, EngineError> {
        let data = PipelineData::new();

        let mut set: JoinSet<Result<crate::engine::Loaded, EngineError>> = JoinSet::new();
        let mut plugin_layers = Vec::new();
        for plugin in &cfg.plugins.value {
            let cfg_value = substitute_config(&plugin.config, &data);
            let config_view = value_to_json(&cfg_value);
            plugin_layers.push(cfg_value);

            let wasmtime = self.wasmtime.clone();
            let cache = self.cache.clone();
            let offline = opts.offline;
            let tag_ttl = self.tag_ttl;
            let wd = opts.working_directory.clone();
            let env2 = env.clone();
            let name = plugin.name.clone();
            let url = crate::engine::plugin_url_string(&plugin.url.value);
            let permissions = crate::engine::effective_permissions(&plugin.permissions);
            let ev = events.clone();
            set.spawn(async move {
                crate::engine::resolve_instantiate_init(
                    wasmtime,
                    cache,
                    offline,
                    tag_ttl,
                    wd,
                    env2,
                    name,
                    url,
                    permissions,
                    config_view,
                    ev,
                )
                .await
            });
        }

        let mut loaded_map: std::collections::HashMap<String, crate::engine::Loaded> =
            std::collections::HashMap::new();
        while let Some(joined) = set.join_next().await {
            match joined {
                Ok(Ok(l)) => {
                    loaded_map.insert(l.name.clone(), l);
                }
                Ok(Err(e)) => {
                    set.shutdown().await;
                    return Err(e);
                }
                Err(join_err) => {
                    set.shutdown().await;
                    return Err(EngineError::Internal(anyhow::anyhow!(
                        "plugin load task failed: {join_err}"
                    )));
                }
            }
        }

        let mut loaded: IndexMap<String, crate::engine::Loaded> = IndexMap::new();
        for plugin in &cfg.plugins.value {
            if let Some(l) = loaded_map.remove(&plugin.name) {
                loaded.insert(plugin.name.clone(), l);
            }
        }

        let mut acc = Accumulator::new();
        acc.push(base);
        acc.push(release);
        for layer in plugin_layers {
            acc.push(layer);
        }

        let src = crate::config::diagnostic::Source::new(content, &opts.config_file_name);
        let mut flat = Vec::new();
        for stage in &cfg.stages.value {
            for step in &stage.steps {
                let run = &step.run.value;
                let l = loaded.get(&run.plugin).ok_or_else(|| {
                    EngineError::Config(src.plugin_not_found(&run.plugin, step.run.span))
                })?;
                if !l.middlewares.iter().any(|m| m == &run.middleware) {
                    return Err(EngineError::Config(
                        src.middleware_not_found(&run.middleware, step.run.span),
                    ));
                }
                flat.push(FlatStep {
                    stage: stage.name.clone(),
                    name: step.name.clone(),
                    plugin: run.plugin.clone(),
                    middleware: run.middleware.clone(),
                    condition: step.condition.clone(),
                    halt_if: step.halt_if.clone(),
                    continue_on_error: step.continue_on_error,
                    config: step.config.clone(),
                });
            }
        }

        let steps = if opts.stages_filter.is_empty() {
            flat
        } else {
            let wanted: Vec<String> = opts
                .stages_filter
                .iter()
                .map(|s| s.to_lowercase())
                .collect();
            flat.into_iter()
                .filter(|f| wanted.contains(&f.stage.to_lowercase()))
                .collect()
        };

        let mut plugins = IndexMap::new();
        let mut plugin_meta = IndexMap::new();
        for (name, l) in loaded {
            plugin_meta.insert(name.clone(), l.meta);
            plugins.insert(name, l.instance);
        }
        Ok(Pipeline {
            plugins,
            steps,
            acc,
            working_directory: opts.working_directory,
            step_timeout: opts.step_timeout,
            plugin_meta,
        })
    }

    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    pub fn plugin_names(&self) -> Vec<&str> {
        self.plugins.keys().map(String::as_str).collect()
    }
}
