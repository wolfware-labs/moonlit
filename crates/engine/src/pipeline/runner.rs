use crate::pipeline::expr::{Value, evaluate_condition, evaluate_halt, substitute_config};
use crate::pipeline::model::FlatStep;
use crate::pipeline::{Pipeline, PipelineError, PipelineEvent, PipelineSummary, StepResult};
use crate::plugin::PluginError;
use crate::plugin::host::ReleaseContext;
use crate::plugin::middleware::MiddlewareResult;
use indexmap::IndexMap;
use std::collections::HashSet;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;

const SEED_WARNING: &str = "No middlewares registered in the pipeline.";

#[must_use = "an execution outcome must be settled into a step result"]
enum ExecOutcome {
  Completed(Result<MiddlewareResult, PluginError>),
  Cancelled,
  TimedOut,
}

#[must_use = "the step flow decides whether the run continues"]
enum StepFlow {
  Continue,
  Halt,
  Stop(PipelineError),
}

#[must_use = "a settled step must be recorded"]
struct Settled {
  successful: bool,
  error_message: Option<String>,
  warnings: Vec<String>,
  halted: bool,
}

struct RunState {
  events: Sender<PipelineEvent>,
  results: Vec<StepResult>,
  warnings: Vec<String>,
  poisoned: HashSet<String>,
  any_executed: bool,
  successful: bool,
}

impl Pipeline {
  pub async fn run(
    mut self,
    events: Sender<PipelineEvent>,
    cancel: CancellationToken,
  ) -> Result<PipelineSummary, PipelineError> {
    let started = Instant::now();
    let steps = std::mem::take(&mut self.steps);
    let total = steps.len();
    let mut state = RunState::new(events);
    let mut halted = false;
    let mut failure = None;

    for (index, step) in steps.iter().enumerate() {
      if cancel.is_cancelled() {
        failure = Some(PipelineError::Execution("Pipeline execution was cancelled.".to_string()));
        break;
      }
      state
        .emit(PipelineEvent::StepStarted {
          index,
          total,
          stage: step.stage.clone(),
          name: step.name.clone(),
          run: format!("{}.{}", step.plugin, step.middleware),
        })
        .await;

      match self.run_step(step, &mut state, &cancel).await {
        StepFlow::Continue => {}
        StepFlow::Halt => {
          halted = true;
          state
            .emit(PipelineEvent::PipelineHalted {
              after_step: step.name.clone(),
              halt_if: step.halt_if.clone().unwrap_or_default(),
            })
            .await;
          break;
        }
        StepFlow::Stop(error) => {
          failure = Some(error);
          break;
        }
      }
    }

    state.finish(started, halted, failure).await
  }

  async fn run_step(&mut self, step: &FlatStep, state: &mut RunState, cancel: &CancellationToken) -> StepFlow {
    if let Some(condition) = &step.condition {
      let outcome = evaluate_condition(condition, &self.data);
      state.warnings.extend(outcome.warning.clone());
      if !outcome.value {
        state
          .emit(PipelineEvent::StepSkipped {
            step: step.name.clone(),
            condition: condition.clone(),
          })
          .await;
        state.record(StepResult::skipped(&step.name, outcome.warning)).await;
        return StepFlow::Continue;
      }
    }

    if state.poisoned.contains(&step.plugin) {
      let message = format!("Plugin '{}' unavailable after an earlier failure in this run.", step.plugin);
      state.record(StepResult::failed(&step.name, Duration::ZERO, &message)).await;
      return if step.continue_on_error {
        StepFlow::Continue
      } else {
        StepFlow::Stop(PipelineError::Execution(message))
      };
    }

    let config = substitute_config(&step.config, &self.data);
    let config_json = config.to_json();
    self.data.push(config);
    state.any_executed = true;
    let started = Instant::now();

    match self.execute(step, &config_json, cancel).await {
      ExecOutcome::Cancelled => StepFlow::Stop(PipelineError::Execution(
        "Pipeline execution was cancelled by the user.".to_string(),
      )),
      ExecOutcome::TimedOut => {
        let timeout = self.step_timeout.expect("only a configured timeout can elapse");
        let message = format!("Step '{}' timed out after {:?}", step.name, timeout);
        state
          .record(StepResult::failed(&step.name, started.elapsed(), &message))
          .await;
        StepFlow::Stop(PipelineError::Execution(message))
      }
      ExecOutcome::Completed(result) => {
        let settled = self.settle(step, result, &mut state.poisoned);
        state.warnings.extend(settled.warnings.iter().cloned());
        state
          .record(StepResult {
            name: step.name.clone(),
            successful: settled.successful,
            skipped: false,
            duration: started.elapsed(),
            error_message: settled.error_message.clone(),
            warnings: settled.warnings,
          })
          .await;

        if !settled.successful && !step.continue_on_error {
          StepFlow::Stop(PipelineError::Execution(format!(
            "An error occurred while executing middleware {}: {}",
            step.middleware,
            settled.error_message.unwrap_or_default()
          )))
        } else if settled.halted {
          StepFlow::Halt
        } else {
          StepFlow::Continue
        }
      }
    }
  }

  async fn execute(&mut self, step: &FlatStep, config: &serde_json::Value, cancel: &CancellationToken) -> ExecOutcome {
    let context = ReleaseContext {
      working_directory: self.working_directory.display().to_string(),
      step_name: step.name.clone(),
    };
    let timeout = self.step_timeout;
    let instance = self
      .plugins
      .get_mut(&step.plugin)
      .expect("load resolves every plugin a step runs");
    let deadline = async move {
      match timeout {
        Some(duration) => tokio::time::sleep(duration).await,
        None => std::future::pending().await,
      }
    };

    tokio::select! {
      biased;
      () = cancel.cancelled() => ExecOutcome::Cancelled,
      result = instance.execute(&step.middleware, context, config) => ExecOutcome::Completed(result),
      () = deadline => ExecOutcome::TimedOut,
    }
  }

  fn settle(
    &mut self,
    step: &FlatStep,
    result: Result<MiddlewareResult, PluginError>,
    poisoned: &mut HashSet<String>,
  ) -> Settled {
    let result = match result {
      Ok(result) => result,
      Err(error) => {
        if matches!(error, PluginError::Trap { .. }) {
          poisoned.insert(step.plugin.clone());
        }
        return Settled {
          successful: false,
          error_message: Some(error.to_string()),
          warnings: Vec::new(),
          halted: false,
        };
      }
    };

    let mut settled = Settled {
      successful: result.successful,
      error_message: result.error_message,
      warnings: result.warnings,
      halted: false,
    };
    if !settled.successful {
      return settled;
    }
    if let Err(message) = self.push_output(&step.name, &result.output) {
      settled.successful = false;
      settled.error_message = Some(message);
      return settled;
    }
    if let Some(halt_if) = &step.halt_if {
      match evaluate_halt(halt_if, &self.data) {
        Ok(halted) => settled.halted = halted,
        Err(error) => {
          settled.successful = false;
          settled.error_message = Some(error.message().to_string());
        }
      }
    }
    settled
  }

  fn push_output(&mut self, step_name: &str, output: &[(String, serde_json::Value)]) -> Result<(), String> {
    let mut values = IndexMap::new();
    for (key, value) in output {
      if values.insert(key.clone(), Value::from_json(value)).is_some() {
        return Err(format!("Key '{key}' already exists"));
      }
    }
    if !values.is_empty() {
      let step_output = IndexMap::from([(step_name.to_string(), Value::Map(values))]);
      self
        .data
        .push(Value::Map(IndexMap::from([("output".to_string(), Value::Map(step_output))])));
    }
    Ok(())
  }
}

impl RunState {
  #[must_use]
  fn new(events: Sender<PipelineEvent>) -> Self {
    Self {
      events,
      results: Vec::new(),
      warnings: Vec::new(),
      poisoned: HashSet::new(),
      any_executed: false,
      successful: true,
    }
  }

  async fn emit(&self, event: PipelineEvent) {
    let _ = self.events.send(event).await;
  }

  async fn record(&mut self, result: StepResult) {
    self.successful &= result.successful;
    self
      .emit(PipelineEvent::StepFinished {
        step: result.name.clone(),
        result: result.clone(),
      })
      .await;
    self.results.push(result);
  }

  async fn finish(
    mut self,
    started: Instant,
    halted: bool,
    failure: Option<PipelineError>,
  ) -> Result<PipelineSummary, PipelineError> {
    if !self.any_executed && failure.is_none() {
      self.warnings.push(SEED_WARNING.to_string());
    }
    let summary = PipelineSummary {
      steps: std::mem::take(&mut self.results),
      successful: failure.is_none() && self.successful,
      halted,
      total_duration: started.elapsed(),
      warnings: std::mem::take(&mut self.warnings),
    };
    self
      .emit(PipelineEvent::PipelineFinished {
        summary: summary.clone(),
      })
      .await;

    match failure {
      Some(error) => Err(error),
      None => Ok(summary),
    }
  }
}

impl StepResult {
  #[must_use]
  fn skipped(name: &str, warning: Option<String>) -> Self {
    Self {
      name: name.to_string(),
      successful: true,
      skipped: true,
      duration: Duration::ZERO,
      error_message: None,
      warnings: warning.into_iter().collect(),
    }
  }

  #[must_use]
  fn failed(name: &str, duration: Duration, message: &str) -> Self {
    Self {
      name: name.to_string(),
      successful: false,
      skipped: false,
      duration,
      error_message: Some(message.to_string()),
      warnings: Vec::new(),
    }
  }
}
