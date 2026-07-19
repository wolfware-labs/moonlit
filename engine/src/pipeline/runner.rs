//! The pipeline runner (§3.1): drives a loaded `Pipeline` step-by-step, streams events, and
//! returns a `PipelineSummary`. Boundary cancellation, during-execute cancellation, and
//! step-timeout (a fatal abort) are all handled here. See the Phase-7 design doc.

use std::time::{Duration, Instant};

use indexmap::IndexMap;
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;

use crate::engine::EngineError;
use crate::expr::value::Value;
use crate::expr::{evaluate_condition, evaluate_halt, substitute_config};
use crate::host::{ReleaseContext, json_to_value, value_to_json};
use crate::pipeline::{Pipeline, PipelineEvent, PipelineSummary, StepResult};

const SEED_WARNING: &str = "No middlewares registered in the pipeline.";

enum ExecOutcome {
    Completed(Result<crate::host::MiddlewareResult, crate::host::HostError>),
    Cancelled,
    TimedOut,
}

pub(crate) async fn run_pipeline(
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
        // 1. Boundary cancellation.
        if cancel.is_cancelled() {
            terminal_err = Some(EngineError::Execution(
                "Pipeline execution was cancelled.".to_string(),
            ));
            break;
        }

        // 2. StepStarted.
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

        // 3. Condition — skip when falsy.
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

        // A plugin that trapped earlier poisoned its wasm Store (wasmtime denies re-entry for the
        // whole run). Fail fast instead of re-entering; the pipeline still continues on other plugins.
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

        // 4. Step config: substitute against the accumulator, push as a layer (§5.2.4).
        let cfg_value = substitute_config(&step.config, &acc);
        acc.push(cfg_value.clone());
        let cfg_json = value_to_json(&cfg_value);

        // 5. Execute.
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
                // In-flight cancel: drop the future (poisons the store, never reused) and stop.
                terminal_err = Some(EngineError::Execution(
                    "Pipeline execution was cancelled by the user.".to_string(),
                ));
                break;
            }
            ExecOutcome::TimedOut => {
                // Fatal abort regardless of continueOnError (poisoned store, never reused).
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

        // 6. Classify.
        let mut successful;
        let mut error_message: Option<String>;
        let step_warnings: Vec<String>;
        match exec {
            Ok(res) => {
                successful = res.successful;
                error_message = res.error_message.clone();
                step_warnings = res.warnings.clone();

                // 7. On success: append outputs (dup key fails), then haltIf.
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
                // Only a real wasm trap poisons the Store (wasmtime denies re-entry). A non-trap
                // error (e.g. HostError::BadJson from an Ok call returning invalid-JSON output)
                // leaves the Store healthy, so the plugin stays usable for later steps.
                if matches!(host_err, crate::host::HostError::Trap { .. }) {
                    poisoned.insert(step.plugin.clone());
                }
                successful = false;
                error_message = Some(host_err.to_string());
                step_warnings = Vec::new();
            }
        }

        // 8. Finalize + StepFinished.
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

        // 9. Terminal decisions.
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

    // Single finalization: seed warning only on a clean idle run; PipelineFinished always emitted.
    if !any_executed && terminal_err.is_none() {
        warnings.push(SEED_WARNING.to_string());
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
