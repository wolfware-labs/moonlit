use crate::engine::Engine;
use crate::pipeline::config::{ConfigDiagnostic, ConfigSource, Permissions, PipelineConfig, PluginUrl, parse_config};
use crate::pipeline::expr::substitute_config;
use crate::pipeline::manifest::PipelineManifest;
use crate::pipeline::model::{ChannelSink, FlatStep};
use crate::pipeline::{IncompatiblePlugin, Pipeline, PipelineData, PipelineError, PipelineEvent, PipelineOptions};
use crate::plugin::host::HostEventSink;
use crate::plugin::publish::PLUGIN_WORLD;
use crate::plugin::resolver::{PluginSource, ProgressFn, ResolveOptions, resolve};
use crate::plugin::world::component_world;
use crate::plugin::{Plugin, PluginInstance, PluginInstanceConfig};
use indexmap::IndexMap;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tokio::task::JoinSet;

struct PluginRequest {
  name: String,
  url: String,
  permissions: Permissions,
  config_view: serde_json::Value,
}

struct ResolvedPlugin {
  request: PluginRequest,
  bytes: Vec<u8>,
  cached: bool,
}

struct LoadedPlugin {
  name: String,
  instance: PluginInstance,
  middlewares: Vec<String>,
}

impl Pipeline {
  pub async fn load(
    engine: &Engine,
    manifest: &PipelineManifest,
    opts: PipelineOptions,
    events: &Sender<PipelineEvent>,
  ) -> Result<Self, PipelineError> {
    let source = ConfigSource::new(&manifest.content, &manifest.file_name);
    let mut config = parse_config(source.yaml, source.name)?;
    for (key, value) in &opts.cli_args {
      config.add_argument(key, value);
    }

    let mut data = PipelineData::new(&config, &manifest.working_dir);
    let mut requests = Vec::new();
    let mut plugin_layers = Vec::new();
    for plugin in &config.plugins.value {
      let layer = substitute_config(&plugin.config, &data);
      requests.push(PluginRequest {
        name: plugin.name.clone(),
        url: plugin_url(&plugin.url.value),
        permissions: plugin.permissions.clone().unwrap_or_else(Permissions::deny),
        config_view: layer.to_json(),
      });
      plugin_layers.push(layer);
    }
    for layer in plugin_layers {
      data.push(layer);
    }

    let plugins = load_plugins(engine, requests, &manifest.working_dir, opts.offline, events).await?;
    let steps = flatten_steps(&config, &plugins, &source, &opts.stages_filter)?;

    Ok(Pipeline {
      plugins: plugins.into_iter().map(|(name, plugin)| (name, plugin.instance)).collect(),
      steps,
      working_directory: manifest.working_dir.clone(),
      step_timeout: opts.step_timeout,
      data,
    })
  }
}

async fn load_plugins(
  engine: &Engine,
  requests: Vec<PluginRequest>,
  working_dir: &Path,
  offline: bool,
  events: &Sender<PipelineEvent>,
) -> Result<IndexMap<String, LoadedPlugin>, PipelineError> {
  let declared: Vec<String> = requests.iter().map(|request| request.name.clone()).collect();
  let env_snapshot: Vec<(String, String)> = std::env::vars().collect();

  let resolved = resolve_plugins(engine, requests, offline, events).await?;
  reject_incompatible(&resolved)?;

  let mut tasks = JoinSet::new();
  for plugin in resolved {
    let engine = engine.clone();
    let working_dir = working_dir.to_path_buf();
    let env_snapshot = env_snapshot.clone();
    let events = events.clone();
    tasks.spawn(async move { load_plugin(&engine, plugin, working_dir, env_snapshot, &events).await });
  }

  let mut loaded: HashMap<String, LoadedPlugin> = join_all(tasks)
    .await?
    .into_iter()
    .map(|plugin| (plugin.name.clone(), plugin))
    .collect();
  Ok(
    declared
      .into_iter()
      .filter_map(|name| loaded.remove(&name).map(|plugin| (name, plugin)))
      .collect(),
  )
}

async fn join_all<T: 'static>(mut tasks: JoinSet<Result<T, PipelineError>>) -> Result<Vec<T>, PipelineError> {
  let mut done = Vec::new();
  while let Some(joined) = tasks.join_next().await {
    let result = match joined {
      Ok(result) => result,
      Err(join_error) => Err(PipelineError::Internal(anyhow::anyhow!(
        "plugin load task failed: {join_error}"
      ))),
    };
    match result {
      Ok(value) => done.push(value),
      Err(error) => {
        tasks.shutdown().await;
        return Err(error);
      }
    }
  }
  Ok(done)
}

async fn resolve_plugins(
  engine: &Engine,
  requests: Vec<PluginRequest>,
  offline: bool,
  events: &Sender<PipelineEvent>,
) -> Result<Vec<ResolvedPlugin>, PipelineError> {
  let order: Vec<String> = requests.iter().map(|request| request.name.clone()).collect();
  let mut tasks = JoinSet::new();
  for request in requests {
    let engine = engine.clone();
    let events = events.clone();
    tasks.spawn(async move { resolve_plugin(&engine, request, offline, &events).await });
  }
  let mut resolved = join_all(tasks).await?;
  resolved.sort_by_key(|plugin| order.iter().position(|name| *name == plugin.request.name));
  Ok(resolved)
}

fn reject_incompatible(resolved: &[ResolvedPlugin]) -> Result<(), PipelineError> {
  let mut incompatible = Vec::new();
  for plugin in resolved {
    let plugin_world = component_world(&plugin.bytes).map_err(|e| PipelineError::PluginLoad {
      plugin: plugin.request.name.clone(),
      message: e.to_string(),
    })?;
    if plugin_world.as_deref() != Some(PLUGIN_WORLD) {
      incompatible.push(IncompatiblePlugin {
        name: plugin.request.name.clone(),
        url: plugin.request.url.clone(),
        plugin_world,
        engine_world: PLUGIN_WORLD.to_string(),
      });
    }
  }
  if incompatible.is_empty() {
    Ok(())
  } else {
    Err(PipelineError::IncompatiblePlugins(incompatible))
  }
}

async fn resolve_plugin(
  engine: &Engine,
  request: PluginRequest,
  offline: bool,
  events: &Sender<PipelineEvent>,
) -> Result<ResolvedPlugin, PipelineError> {
  let name = request.name.clone();
  let url = request.url.clone();
  let load_error = |message: String| PipelineError::PluginLoad {
    plugin: name.clone(),
    message,
  };

  let _ = events
    .send(PipelineEvent::PluginResolving {
      name: name.clone(),
      url: url.clone(),
    })
    .await;

  let source = url.parse::<PluginSource>().map_err(|e| load_error(e.to_string()))?;
  let options = ResolveOptions {
    offline,
    tag_ttl: engine.tag_ttl(),
  };
  let report_progress = |received: u64, total: Option<u64>| {
    let _ = events.try_send(PipelineEvent::PluginPullProgress {
      name: name.clone(),
      received,
      total,
    });
  };
  let progress: ProgressFn = &report_progress;
  let resolved = resolve(&source, &options, engine.cache(), Some(progress))
    .await
    .map_err(|e| load_error(e.to_string()))?;

  let bytes =
    std::fs::read(&resolved.wasm_path).map_err(|e| load_error(format!("reading {}: {e}", resolved.wasm_path.display())))?;

  Ok(ResolvedPlugin {
    request,
    bytes,
    cached: resolved.cached,
  })
}

async fn load_plugin(
  engine: &Engine,
  plugin: ResolvedPlugin,
  working_dir: PathBuf,
  env_snapshot: Vec<(String, String)>,
  events: &Sender<PipelineEvent>,
) -> Result<LoadedPlugin, PipelineError> {
  let ResolvedPlugin { request, bytes, cached } = plugin;
  let PluginRequest {
    name,
    permissions,
    config_view,
    ..
  } = request;
  let load_error = |message: String| PipelineError::PluginLoad {
    plugin: name.clone(),
    message,
  };

  let instance_config = PluginInstanceConfig {
    working_directory: working_dir,
    permissions,
    config_view: config_view.clone(),
    env_snapshot,
  };
  let sink: Arc<dyn HostEventSink> = Arc::new(ChannelSink { events: events.clone() });

  let mut instance = Plugin::instantiate(engine, &bytes, instance_config, sink)
    .await
    .map_err(|e| load_error(e.to_string()))?;
  let metadata = instance.init(&config_view).await.map_err(load_error)?;
  let middlewares = instance
    .list_middlewares()
    .await
    .map_err(|e| load_error(e.to_string()))?
    .into_iter()
    .map(|middleware| middleware.name)
    .collect();

  let _ = events
    .send(PipelineEvent::PluginReady {
      name: name.clone(),
      version: metadata.version,
      cached,
    })
    .await;

  Ok(LoadedPlugin {
    name,
    instance,
    middlewares,
  })
}

fn flatten_steps(
  config: &PipelineConfig,
  plugins: &IndexMap<String, LoadedPlugin>,
  source: &ConfigSource,
  stages_filter: &[String],
) -> Result<Vec<FlatStep>, PipelineError> {
  let wanted: Vec<String> = stages_filter.iter().map(|stage| stage.to_lowercase()).collect();
  let mut steps = Vec::new();
  for stage in &config.stages.value {
    let selected = wanted.is_empty() || wanted.contains(&stage.name.to_lowercase());
    for step in &stage.steps {
      let run = &step.run.value;
      let plugin = plugins.get(&run.plugin).expect("parse_config rejects undeclared plugins");
      if !plugin.middlewares.contains(&run.middleware) {
        return Err(ConfigDiagnostic::middleware_not_found(source, &run.middleware, step.run.span).into());
      }
      if selected {
        steps.push(FlatStep {
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
  }
  Ok(steps)
}

#[must_use]
fn plugin_url(url: &PluginUrl) -> String {
  match url {
    PluginUrl::Oci(s) | PluginUrl::File(s) | PluginUrl::Http(s) | PluginUrl::Https(s) => s.clone(),
  }
}
