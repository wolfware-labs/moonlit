#![allow(clippy::result_large_err)]

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use indexmap::IndexMap;
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;

use crate::cache::{Cache, SystemClock};
use crate::config::model::{Permissions, PluginUrl};
use crate::expr::{Accumulator, substitute_config};
use crate::host::{HostEventSink, InstanceConfig, PluginInstance, PluginMetadata, value_to_json};
use crate::pipeline::{ChannelSink, FlatStep, Pipeline, PipelineEvent, PipelineSummary};
use crate::resolve::{self, PluginSource, ResolveOptions};

pub struct EngineSettings {
    pub cache_dir: Option<PathBuf>,
    pub tag_ttl: Duration,
}

impl Default for EngineSettings {
    fn default() -> Self {
        Self {
            cache_dir: None,
            tag_ttl: Duration::from_secs(15 * 60),
        }
    }
}

pub struct PipelineOptions {
    pub working_directory: PathBuf,
    pub config_file_name: String,
    pub stages_filter: Vec<String>,
    pub cli_args: Vec<(String, String)>,
    pub step_timeout: Option<Duration>,
    pub offline: bool,
}

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum EngineError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Config(#[from] crate::config::ConfigDiagnostic),

    #[error("failed to load plugin '{plugin}': {message}")]
    #[diagnostic(code(moonlit::engine::plugin_load))]
    PluginLoad { plugin: String, message: String },

    #[error("pipeline execution failed: {0}")]
    #[diagnostic(code(moonlit::engine::execution))]
    Execution(String),

    #[error(transparent)]
    #[diagnostic(code(moonlit::engine::internal))]
    Internal(#[from] anyhow::Error),
}

impl EngineError {
    pub fn exit_code(&self) -> i32 {
        match self {
            EngineError::Config(_) => 2,
            EngineError::PluginLoad { .. } => 3,
            EngineError::Execution(_) => 4,
            EngineError::Internal(_) => 1,
        }
    }
}

pub struct Engine {
    pub(crate) wasmtime: wasmtime::Engine,
    pub(crate) cache: Arc<Cache>,
    pub(crate) tag_ttl: Duration,
}

impl Engine {
    pub fn new(settings: EngineSettings) -> Result<Self, EngineError> {
        let wasmtime = crate::host::build_engine().map_err(EngineError::Internal)?;
        let cache = match settings.cache_dir {
            Some(dir) => Cache::with_root_and_clock(dir, Box::new(SystemClock)),
            None => Cache::new().map_err(|e| EngineError::Internal(e.into()))?,
        };
        Ok(Self {
            wasmtime,
            cache: Arc::new(cache),
            tag_ttl: settings.tag_ttl,
        })
    }
}

struct Loaded {
    name: String,
    instance: PluginInstance,
    meta: PluginMetadata,
    middlewares: Vec<String>,
}

fn effective_permissions(p: &Option<Permissions>) -> Permissions {
    p.clone().unwrap_or_else(Permissions::deny)
}

fn plugin_url_string(u: &PluginUrl) -> String {
    match u {
        PluginUrl::Oci(s) | PluginUrl::File(s) | PluginUrl::Http(s) | PluginUrl::Https(s) => {
            s.clone()
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn resolve_instantiate_init(
    wasmtime: wasmtime::Engine,
    cache: Arc<Cache>,
    offline: bool,
    tag_ttl: Duration,
    working_directory: PathBuf,
    env_snapshot: Vec<(String, String)>,
    name: String,
    url: String,
    permissions: Permissions,
    config_view: serde_json::Value,
    events: Sender<PipelineEvent>,
) -> Result<Loaded, EngineError> {
    let load_err = |message: String| EngineError::PluginLoad {
        plugin: name.clone(),
        message,
    };

    let _ = events
        .send(PipelineEvent::PluginResolving {
            name: name.clone(),
            url: url.clone(),
        })
        .await;

    let source = PluginSource::parse(&url).map_err(|e| load_err(e.to_string()))?;
    let ropts = ResolveOptions { offline, tag_ttl };

    let ev = events.clone();
    let nm = name.clone();
    let progress = move |received: u64, total: Option<u64>| {
        let _ = ev.try_send(PipelineEvent::PluginPullProgress {
            name: nm.clone(),
            received,
            total,
        });
    };
    let progress_fn: &(dyn Fn(u64, Option<u64>) + Send + Sync) = &progress;

    let resolved = resolve::resolve(&source, &ropts, cache.as_ref(), Some(progress_fn))
        .await
        .map_err(|e| load_err(e.to_string()))?;

    let bytes = std::fs::read(&resolved.wasm_path)
        .map_err(|e| load_err(format!("reading {}: {e}", resolved.wasm_path.display())))?;

    let inst_cfg = InstanceConfig {
        working_directory,
        permissions,
        config_view: config_view.clone(),
        env_snapshot,
    };
    let sink: Arc<dyn HostEventSink> = Arc::new(ChannelSink {
        events: events.clone(),
    });

    let mut instance = PluginInstance::instantiate(&wasmtime, &bytes, inst_cfg, sink)
        .await
        .map_err(|e| load_err(e.to_string()))?;
    let meta = instance.init(&config_view).await.map_err(load_err)?;
    let middlewares = instance
        .list_middlewares()
        .await
        .map_err(|e| load_err(e.to_string()))?
        .into_iter()
        .map(|m| m.name)
        .collect();

    let _ = events
        .send(PipelineEvent::PluginReady {
            name: name.clone(),
            version: meta.version.clone(),
            cached: resolved.cached,
        })
        .await;

    Ok(Loaded {
        name,
        instance,
        meta,
        middlewares,
    })
}

impl Engine {
    pub async fn load_pipeline(
        &self,
        yaml: &str,
        opts: PipelineOptions,
        events: &Sender<PipelineEvent>,
    ) -> Result<Pipeline, EngineError> {
        let cfg = crate::config::parse_config(yaml, &opts.config_file_name)?;

        let env: Vec<(String, String)> = std::env::vars().collect();
        let dotenv = std::fs::read_to_string(opts.working_directory.join(".env")).ok();
        let base = Accumulator::build_base_layer(&env, dotenv.as_deref());
        let release =
            Accumulator::build_release_layer(&cfg.variables, &cfg.arguments, &opts.cli_args);

        let mut subst_acc = Accumulator::new();
        subst_acc.push(base.clone());
        subst_acc.push(release.clone());

        let mut set: tokio::task::JoinSet<Result<Loaded, EngineError>> =
            tokio::task::JoinSet::new();
        let mut plugin_layers = Vec::new();
        for plugin in &cfg.plugins.value {
            let cfg_value = substitute_config(&plugin.config, &subst_acc);
            let config_view = value_to_json(&cfg_value);
            plugin_layers.push(cfg_value);

            let wasmtime = self.wasmtime.clone();
            let cache = self.cache.clone();
            let offline = opts.offline;
            let tag_ttl = self.tag_ttl;
            let wd = opts.working_directory.clone();
            let env2 = env.clone();
            let name = plugin.name.clone();
            let url = plugin_url_string(&plugin.url.value);
            let permissions = effective_permissions(&plugin.permissions);
            let ev = events.clone();
            set.spawn(async move {
                resolve_instantiate_init(
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

        let mut loaded_map: std::collections::HashMap<String, Loaded> =
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

        let mut loaded: IndexMap<String, Loaded> = IndexMap::new();
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

        let src = crate::config::diagnostic::Source::new(yaml, &opts.config_file_name);
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

    pub async fn run(
        &self,
        pipeline: Pipeline,
        events: Sender<PipelineEvent>,
        cancel: CancellationToken,
    ) -> Result<PipelineSummary, EngineError> {
        crate::pipeline::run_pipeline(pipeline, events, cancel).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omitted_permissions_defaults_to_deny() {
        assert_eq!(effective_permissions(&None), Permissions::deny());
    }

    #[test]
    fn present_permissions_are_used_verbatim() {
        let p = Permissions::deny();
        assert_eq!(effective_permissions(&Some(p.clone())), p);
    }
}
