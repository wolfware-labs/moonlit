use crate::host::HostEventSink;
use crate::logging::LogLevel;
use serde::Serialize;
use std::path::PathBuf;
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
    #[serde(
        rename = "duration_ms",
        serialize_with = "serializers::serialize_duration_ms"
    )]
    pub duration: Duration,
    pub error_message: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PipelineSummary {
    pub steps: Vec<StepResult>,
    pub successful: bool,
    pub halted: bool,
    #[serde(
        rename = "total_duration_ms",
        serialize_with = "serializers::serialize_duration_ms"
    )]
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
    pub working_directory: PathBuf,
    pub config_file_name: String,
    pub stages_filter: Vec<String>,
    pub cli_args: Vec<(String, String)>,
    pub step_timeout: Option<Duration>,
    pub offline: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MiddlewareResult {
    pub successful: bool,
    pub error_message: Option<String>,
    pub warnings: Vec<String>,
    pub output: Vec<(String, serde_json::Value)>,
}

pub struct InstanceConfig {
    pub working_directory: PathBuf,
    pub permissions: crate::config::model::Permissions,
    pub config_view: serde_json::Value,
    pub env_snapshot: Vec<(String, String)>,
}

// impl Pipeline {
//     pub fn step_count(&self) -> usize {
//         self.steps.len()
//     }
//     pub fn plugin_names(&self) -> Vec<&str> {
//         self.plugins.keys().map(String::as_str).collect()
//     }
// }

mod serializers {
    use serde::Serializer;
    use std::time::Duration;

    pub fn serialize_duration_ms<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u64(d.as_millis() as u64)
    }
}
