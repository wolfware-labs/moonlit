pub mod config;
pub mod error;

use crate::cache::{Cache, SystemClock};
use crate::engine::config::EngineSettings;
use crate::engine::error::EngineError;
use crate::host::state::HostState;
use indexmap::IndexMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;
use wasmtime::component::{HasSelf, Linker};

const SEED_WARNING: &str = "No middlewares registered in the pipeline.";

enum ExecOutcome {
    Completed(Result<crate::host::MiddlewareResult, crate::host::HostError>),
    Cancelled,
    TimedOut,
}

pub struct Engine {
    wasmtime: wasmtime::Engine,
    cache: Arc<Cache>,
    tag_ttl: Duration,
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

    pub fn build_linker(&self) -> anyhow::Result<Linker<HostState>> {
        let mut linker: Linker<HostState> = Linker::new(&self.wasmtime);
        wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
        wasmtime_wasi_http::p2::add_only_http_to_linker_async(&mut linker)?;
        crate::host::wit::moonlit::plugin::host::add_to_linker::<_, HasSelf<_>>(&mut linker, |s| s)?;
        crate::host::wit::moonlit::plugin::process::add_to_linker::<_, HasSelf<_>>(&mut linker, |s| s)?;
        Ok(linker)
    }

    pub async fn run_pipeline(
        &self,
        pipeline: Pipeline,
        events: Sender<PipelineEvent>,
        cancel: CancellationToken,
    ) -> Result<PipelineSummary, EngineError> {
        let Pipeline {
            mut plugins,
            steps,
            mut acc,
            working_directory,
            step_timeout,
            plugin_meta: _,
        } = pipeline;

        let started = Instant::now();
        let total = steps.len();
        let wd = working_directory.display().to_string();

        let mut results: Vec<StepResult> = Vec::new();
        let mut warnings: Vec<String> = Vec::new();
        let mut any_executed = false;
        let mut overall_success = true;
        let mut halted = false;
        let mut terminal_err: Option<EngineError> = None;
        let mut poisoned: std::collections::HashSet<String> = std::collections::HashSet::new();

        for (index, step) in steps.iter().enumerate() {
            if cancel.is_cancelled() {
                terminal_err = Some(EngineError::Execution(
                    "Pipeline execution was cancelled.".to_string(),
                ));
                break;
            }

            let run = format!("{}.{}", step.plugin, step.middleware);
            let _ = events
                .send(PipelineEvent::StepStarted {
                    index,
                    total,
                    stage: step.stage.clone(),
                    name: step.name.clone(),
                    run,
                })
                .await;

            if let Some(cond) = &step.condition {
                let outcome = evaluate_condition(cond, &acc);
                if let Some(w) = &outcome.warning {
                    warnings.push(w.clone());
                }
                if !outcome.value {
                    let _ = events
                        .send(PipelineEvent::StepSkipped {
                            step: step.name.clone(),
                            condition: cond.clone(),
                        })
                        .await;
                    let result = StepResult {
                        name: step.name.clone(),
                        successful: true,
                        skipped: true,
                        duration: Duration::ZERO,
                        error_message: None,
                        warnings: outcome.warning.into_iter().collect(),
                    };
                    let _ = events
                        .send(PipelineEvent::StepFinished {
                            step: step.name.clone(),
                            result: result.clone(),
                        })
                        .await;
                    results.push(result);
                    continue;
                }
            }

            if poisoned.contains(&step.plugin) {
                overall_success = false;
                let msg = format!(
                    "Plugin '{}' unavailable after an earlier failure in this run.",
                    step.plugin
                );
                let result = StepResult {
                    name: step.name.clone(),
                    successful: false,
                    skipped: false,
                    duration: Duration::ZERO,
                    error_message: Some(msg.clone()),
                    warnings: Vec::new(),
                };
                let _ = events
                    .send(PipelineEvent::StepFinished {
                        step: step.name.clone(),
                        result: result.clone(),
                    })
                    .await;
                results.push(result);
                if !step.continue_on_error {
                    terminal_err = Some(EngineError::Execution(msg));
                    break;
                }
                continue;
            }

            let cfg_value = substitute_config(&step.config, &acc);
            acc.push(cfg_value.clone());
            let cfg_json = value_to_json(&cfg_value);

            any_executed = true;
            let step_started = Instant::now();
            let ctx = ReleaseContext {
                working_directory: wd.clone(),
                step_name: step.name.clone(),
            };
            let instance = plugins
                .get_mut(&step.plugin)
                .expect("plugin present (validated at load)");
            let outcome = {
                let fut = instance.execute(&step.middleware, ctx, &cfg_json);
                match step_timeout {
                    Some(to) => tokio::select! {
                        biased;
                        _ = cancel.cancelled() => ExecOutcome::Cancelled,
                        r = fut => ExecOutcome::Completed(r),
                        _ = tokio::time::sleep(to) => ExecOutcome::TimedOut,
                    },
                    None => tokio::select! {
                        biased;
                        _ = cancel.cancelled() => ExecOutcome::Cancelled,
                        r = fut => ExecOutcome::Completed(r),
                    },
                }
            };
            let exec = match outcome {
                ExecOutcome::Completed(r) => r,
                ExecOutcome::Cancelled => {
                    terminal_err = Some(EngineError::Execution(
                        "Pipeline execution was cancelled by the user.".to_string(),
                    ));
                    break;
                }
                ExecOutcome::TimedOut => {
                    overall_success = false;
                    let to = step_timeout.expect("timeout branch only runs when Some");
                    let msg = format!("Step '{}' timed out after {:?}", step.name, to);
                    let result = StepResult {
                        name: step.name.clone(),
                        successful: false,
                        skipped: false,
                        duration: step_started.elapsed(),
                        error_message: Some(msg.clone()),
                        warnings: Vec::new(),
                    };
                    let _ = events
                        .send(PipelineEvent::StepFinished {
                            step: step.name.clone(),
                            result: result.clone(),
                        })
                        .await;
                    results.push(result);
                    terminal_err = Some(EngineError::Execution(msg));
                    break;
                }
            };

            let mut successful;
            let mut error_message: Option<String>;
            let step_warnings: Vec<String>;
            match exec {
                Ok(res) => {
                    successful = res.successful;
                    error_message = res.error_message.clone();
                    step_warnings = res.warnings.clone();

                    if successful {
                        let mut out_map: IndexMap<String, Value> = IndexMap::new();
                        for (key, jval) in &res.output {
                            if out_map.contains_key(key) {
                                successful = false;
                                error_message = Some(format!("Key '{key}' already exists"));
                                break;
                            }
                            out_map.insert(key.clone(), json_to_value(jval));
                        }
                        if successful && !out_map.is_empty() {
                            let mut step_map: IndexMap<String, Value> = IndexMap::new();
                            step_map.insert(step.name.clone(), Value::Map(out_map));
                            let mut root: IndexMap<String, Value> = IndexMap::new();
                            root.insert("output".to_string(), Value::Map(step_map));
                            acc.push(Value::Map(root));
                        }
                        if successful && let Some(h) = &step.halt_if {
                            match evaluate_halt(h, &acc) {
                                Ok(true) => halted = true,
                                Ok(false) => {}
                                Err(e) => {
                                    successful = false;
                                    error_message = Some(e.message().to_string());
                                }
                            }
                        }
                    }
                }
                Err(host_err) => {
                    if matches!(host_err, crate::host::HostError::Trap { .. }) {
                        poisoned.insert(step.plugin.clone());
                    }
                    successful = false;
                    error_message = Some(host_err.to_string());
                    step_warnings = Vec::new();
                }
            }

            if !successful {
                overall_success = false;
            }
            let result = StepResult {
                name: step.name.clone(),
                successful,
                skipped: false,
                duration: step_started.elapsed(),
                error_message: error_message.clone(),
                warnings: step_warnings.clone(),
            };
            let _ = events
                .send(PipelineEvent::StepFinished {
                    step: step.name.clone(),
                    result: result.clone(),
                })
                .await;
            results.push(result);
            warnings.extend(step_warnings);

            if !successful && !step.continue_on_error {
                let msg = error_message.unwrap_or_default();
                terminal_err = Some(EngineError::Execution(format!(
                    "An error occurred while executing middleware {}: {}",
                    step.middleware, msg
                )));
                break;
            }
            if halted {
                let _ = events
                    .send(PipelineEvent::PipelineHalted {
                        after_step: step.name.clone(),
                        halt_if: step.halt_if.clone().unwrap_or_default(),
                    })
                    .await;
                break;
            }
        }

        if !any_executed && terminal_err.is_none() {
            warnings.push(crate::pipeline::runner::SEED_WARNING.to_string());
        }
        let summary = PipelineSummary {
            steps: results,
            successful: terminal_err.is_none() && overall_success,
            halted,
            total_duration: started.elapsed(),
            warnings,
        };
        let _ = events
            .send(PipelineEvent::PipelineFinished {
                summary: summary.clone(),
            })
            .await;

        match terminal_err {
            Some(e) => Err(e),
            None => Ok(summary),
        }
    }
}

//
// struct Loaded {
//     name: String,
//     instance: PluginInstance,
//     meta: PluginMetadata,
//     middlewares: Vec<String>,
// }
//
// fn effective_permissions(p: &Option<Permissions>) -> Permissions {
//     p.clone().unwrap_or_else(Permissions::deny)
// }
//
// fn plugin_url_string(u: &PluginUrl) -> String {
//     match u {
//         PluginUrl::Oci(s) | PluginUrl::File(s) | PluginUrl::Http(s) | PluginUrl::Https(s) => {
//             s.clone()
//         }
//     }
// }
//
// async fn resolve_instantiate_init(
//     wasmtime: wasmtime::Engine,
//     cache: Arc<Cache>,
//     offline: bool,
//     tag_ttl: Duration,
//     working_directory: PathBuf,
//     env_snapshot: Vec<(String, String)>,
//     name: String,
//     url: String,
//     permissions: Permissions,
//     config_view: serde_json::Value,
//     events: Sender<PipelineEvent>,
// ) -> Result<Loaded, EngineError> {
//     let load_err = |message: String| EngineError::PluginLoad {
//         plugin: name.clone(),
//         message,
//     };
//
//     let _ = events
//         .send(PipelineEvent::PluginResolving {
//             name: name.clone(),
//             url: url.clone(),
//         })
//         .await;
//
//     let source = PluginSource::parse(&url).map_err(|e| load_err(e.to_string()))?;
//     let ropts = ResolveOptions { offline, tag_ttl };
//
//     let ev = events.clone();
//     let nm = name.clone();
//     let progress = move |received: u64, total: Option<u64>| {
//         let _ = ev.try_send(PipelineEvent::PluginPullProgress {
//             name: nm.clone(),
//             received,
//             total,
//         });
//     };
//     let progress_fn: &(dyn Fn(u64, Option<u64>) + Send + Sync) = &progress;
//
//     let resolved = resolve::resolve(&source, &ropts, cache.as_ref(), Some(progress_fn))
//         .await
//         .map_err(|e| load_err(e.to_string()))?;
//
//     let bytes = std::fs::read(&resolved.wasm_path)
//         .map_err(|e| load_err(format!("reading {}: {e}", resolved.wasm_path.display())))?;
//
//     let inst_cfg = InstanceConfig {
//         working_directory,
//         permissions,
//         config_view: config_view.clone(),
//         env_snapshot,
//     };
//     let sink: Arc<dyn HostEventSink> = Arc::new(ChannelSink {
//         events: events.clone(),
//     });
//
//     let mut instance = PluginInstance::instantiate(&wasmtime, &bytes, inst_cfg, sink)
//         .await
//         .map_err(|e| load_err(e.to_string()))?;
//     let meta = instance.init(&config_view).await.map_err(load_err)?;
//     let middlewares = instance
//         .list_middlewares()
//         .await
//         .map_err(|e| load_err(e.to_string()))?
//         .into_iter()
//         .map(|m| m.name)
//         .collect();
//
//     let _ = events
//         .send(PipelineEvent::PluginReady {
//             name: name.clone(),
//             version: meta.version.clone(),
//             cached: resolved.cached,
//         })
//         .await;
//
//     Ok(Loaded {
//         name,
//         instance,
//         meta,
//         middlewares,
//     })
// }
