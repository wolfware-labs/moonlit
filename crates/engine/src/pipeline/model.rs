use crate::logging::LogLevel;
use crate::pipeline::config::ConfigMap;
use crate::plugin::host::HostEventSink;
use serde::Serialize;
use std::time::Duration;
use tokio::sync::mpsc::Sender;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PipelineEvent {
  PluginResolving {
    name: String,
    url: String,
  },
  PluginPullProgress {
    name: String,
    received: u64,
    total: Option<u64>,
  },
  PluginReady {
    name: String,
    version: String,
    cached: bool,
  },
  StepStarted {
    index: usize,
    total: usize,
    stage: String,
    name: String,
    run: String,
  },
  StepLog {
    step: String,
    level: LogLevel,
    message: String,
  },
  StepProgress {
    step: String,
    message: String,
  },
  StepSkipped {
    step: String,
    condition: String,
  },
  StepFinished {
    step: String,
    result: StepResult,
  },
  PipelineHalted {
    after_step: String,
    halt_if: String,
  },
  PipelineFinished {
    summary: PipelineSummary,
  },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct StepResult {
  pub name: String,
  pub successful: bool,
  pub skipped: bool,
  #[serde(rename = "duration_ms", serialize_with = "serializers::serialize_duration_ms")]
  pub duration: Duration,
  pub error_message: Option<String>,
  pub warnings: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PipelineSummary {
  pub steps: Vec<StepResult>,
  pub successful: bool,
  pub halted: bool,
  #[serde(rename = "total_duration_ms", serialize_with = "serializers::serialize_duration_ms")]
  pub total_duration: Duration,
  pub warnings: Vec<String>,
}

pub struct ChannelSink {
  pub events: Sender<PipelineEvent>,
}

impl HostEventSink for ChannelSink {
  fn log(&self, step: &str, level: LogLevel, message: &str) {
    let _ = self.events.try_send(PipelineEvent::StepLog {
      step: step.to_string(),
      level,
      message: message.to_string(),
    });
  }
  fn progress(&self, step: &str, message: &str) {
    let _ = self.events.try_send(PipelineEvent::StepProgress {
      step: step.to_string(),
      message: message.to_string(),
    });
  }
}

pub struct FlatStep {
  pub stage: String,
  pub name: String,
  pub plugin: String,
  pub middleware: String,
  pub condition: Option<String>,
  pub halt_if: Option<String>,
  pub continue_on_error: bool,
  pub config: ConfigMap,
}

pub struct PipelineOptions {
  pub stages_filter: Vec<String>,
  pub cli_args: Vec<(String, String)>,
  pub step_timeout: Option<Duration>,
  pub offline: bool,
}

mod serializers {
  use serde::Serializer;
  use std::time::Duration;

  pub fn serialize_duration_ms<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_u64(u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn step(duration: Duration) -> StepResult {
    StepResult {
      name: "s1".to_string(),
      successful: true,
      skipped: false,
      duration,
      error_message: None,
      warnings: vec![],
    }
  }

  #[test]
  fn events_serialize_with_a_type_tag_and_millisecond_durations() {
    let event = PipelineEvent::StepFinished {
      step: "s1".to_string(),
      result: step(Duration::from_millis(1500)),
    };
    let json = serde_json::to_value(&event).unwrap();
    assert_eq!(json["type"], "step_finished");
    assert_eq!(json["result"]["duration_ms"], 1500);
  }

  #[test]
  fn summaries_serialize_the_total_duration_in_milliseconds() {
    let summary = PipelineSummary {
      steps: vec![step(Duration::ZERO)],
      successful: true,
      halted: false,
      total_duration: Duration::from_secs(2),
      warnings: vec![],
    };
    let json = serde_json::to_value(&summary).unwrap();
    assert_eq!(json["total_duration_ms"], 2000);
    assert_eq!(json["steps"][0]["duration_ms"], 0);
  }

  #[test]
  fn durations_beyond_u64_milliseconds_saturate() {
    let json = serde_json::to_value(step(Duration::MAX)).unwrap();
    assert_eq!(json["duration_ms"], u64::MAX);
  }
}
