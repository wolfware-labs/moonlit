use crate::engine::Engine;
use crate::pipeline::config::{ConfigDiagnostic, ConfigSource, Permissions, PipelineConfig, Span, parse_config};
use crate::pipeline::expr::substitute_config;
use crate::pipeline::manifest::PipelineManifest;
use crate::pipeline::model::{ChannelSink, FlatStep};
use crate::pipeline::{IncompatiblePlugin, Pipeline, PipelineData, PipelineError, PipelineEvent, PipelineOptions};
use crate::plugin::host::HostEventSink;
use crate::plugin::permissions::uncovered_required;
use crate::plugin::publish::PLUGIN_WORLD;
use crate::plugin::resolver::{PluginSource, ProgressFn, ResolveOptions, resolve};
use crate::plugin::world::{self, WorldError};
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
  url_span: Span,
  permissions: Permissions,
  config_view: serde_json::Value,
}

struct ResolvedPlugin {
  request: PluginRequest,
  bytes: Vec<u8>,
  cached: bool,
}

struct DescribedPlugin {
  name: String,
  url_span: Span,
  permissions: Permissions,
  instance: PluginInstance,
  config_view: serde_json::Value,
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
        url: plugin.url.value.as_str().to_string(),
        url_span: plugin.url.span,
        permissions: plugin.permissions.clone().unwrap_or_else(Permissions::deny),
        config_view: layer.to_json(),
      });
      plugin_layers.push(layer);
    }
    for layer in plugin_layers {
      data.push(layer);
    }

    let described = describe_plugins(engine, requests, &manifest.working_dir, opts.offline, events).await?;
    reject_ungranted(&described, &source)?;
    let plugins = init_plugins(described, events).await?;
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

async fn describe_plugins(
  engine: &Engine,
  requests: Vec<PluginRequest>,
  working_dir: &Path,
  offline: bool,
  events: &Sender<PipelineEvent>,
) -> Result<IndexMap<String, DescribedPlugin>, PipelineError> {
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
    tasks.spawn(async move { describe_plugin(&engine, plugin, working_dir, env_snapshot, &events).await });
  }

  let described = join_all(tasks).await?.into_iter().map(|plugin| (plugin.name.clone(), plugin));
  Ok(in_order(&declared, described))
}

async fn init_plugins(
  described: IndexMap<String, DescribedPlugin>,
  events: &Sender<PipelineEvent>,
) -> Result<IndexMap<String, LoadedPlugin>, PipelineError> {
  let declared: Vec<String> = described.keys().cloned().collect();
  let mut tasks = JoinSet::new();
  for plugin in described.into_values() {
    let events = events.clone();
    tasks.spawn(async move { init_plugin(plugin, &events).await });
  }
  let loaded = join_all(tasks).await?.into_iter().map(|plugin| (plugin.name.clone(), plugin));
  Ok(in_order(&declared, loaded))
}

fn in_order<T>(declared: &[String], items: impl Iterator<Item = (String, T)>) -> IndexMap<String, T> {
  let mut by_name: HashMap<String, T> = items.collect();
  declared
    .iter()
    .filter_map(|name| by_name.remove(name).map(|item| (name.clone(), item)))
    .collect()
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
    match world::check(&plugin.bytes) {
      Ok(()) => {}
      Err(WorldError::Mismatch(mismatch)) => incompatible.push(IncompatiblePlugin {
        name: plugin.request.name.clone(),
        url: plugin.request.url.clone(),
        plugin_world: mismatch.found,
        engine_world: PLUGIN_WORLD.to_string(),
      }),
      Err(WorldError::Load(e)) => {
        return Err(PipelineError::PluginLoad {
          plugin: plugin.request.name.clone(),
          message: e.to_string(),
        });
      }
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

async fn describe_plugin(
  engine: &Engine,
  plugin: ResolvedPlugin,
  working_dir: PathBuf,
  env_snapshot: Vec<(String, String)>,
  events: &Sender<PipelineEvent>,
) -> Result<DescribedPlugin, PipelineError> {
  let ResolvedPlugin { request, bytes, cached } = plugin;
  let PluginRequest {
    name,
    url_span,
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
    permissions: permissions.clone(),
    config_view: config_view.clone(),
    env_snapshot,
  };
  let sink: Arc<dyn HostEventSink> = Arc::new(ChannelSink { events: events.clone() });

  let mut instance = Plugin::instantiate(engine, &bytes, instance_config, sink)
    .await
    .map_err(|e| load_error(e.to_string()))?;
  let declared = instance.describe().await.map_err(|e| load_error(e.to_string()))?.permissions;
  instance.set_declared(declared);

  Ok(DescribedPlugin {
    name,
    url_span,
    permissions,
    instance,
    config_view,
    cached,
  })
}

async fn init_plugin(plugin: DescribedPlugin, events: &Sender<PipelineEvent>) -> Result<LoadedPlugin, PipelineError> {
  let DescribedPlugin {
    name,
    mut instance,
    config_view,
    cached,
    ..
  } = plugin;
  let load_error = |message: String| PipelineError::PluginLoad {
    plugin: name.clone(),
    message,
  };

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

fn reject_ungranted(plugins: &IndexMap<String, DescribedPlugin>, source: &ConfigSource) -> Result<(), PipelineError> {
  let missing: Vec<ConfigDiagnostic> = plugins
    .values()
    .flat_map(|plugin| {
      uncovered_required(plugin.instance.declared(), &plugin.permissions)
        .into_iter()
        .map(|permission| ConfigDiagnostic::missing_grant(source, &plugin.name, permission, plugin.url_span))
    })
    .collect();
  if missing.is_empty() {
    Ok(())
  } else {
    Err(PipelineError::MissingGrants(missing))
  }
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
