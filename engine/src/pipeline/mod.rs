//! Shared pipeline data types (the run loop lands here in Phase 7).

use std::path::PathBuf;
use std::time::Duration;

use indexmap::IndexMap;
use serde::Serialize;
use tokio::sync::mpsc::Sender;

use crate::config::model::ConfigMap;
use crate::expr::Accumulator;
use crate::host::{HostEventSink, LogLevel, PluginInstance, PluginMetadata};

/// Serialize a `Duration` as integer milliseconds (stable json contract; matches the
/// human `210ms` rendering). Used via `#[serde(serialize_with = ...)]` on duration fields.
mod duration_ms {
    use serde::Serializer;
    use std::time::Duration;

    pub fn serialize<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u64(d.as_millis() as u64)
    }
}

/// Events streamed to the CLI over an mpsc channel. Phase 6 emits the plugin-load events (and
/// `StepLog`/`StepProgress` while a plugin's `init` runs); the `Step*`/halt/finish events are
/// produced by the Phase-7 runner.
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

/// The outcome of one executed step (produced by the Phase-7 runner).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct StepResult {
    pub name: String,
    pub successful: bool,
    pub skipped: bool,
    #[serde(rename = "duration_ms", serialize_with = "duration_ms::serialize")]
    pub duration: Duration,
    pub error_message: Option<String>,
    pub warnings: Vec<String>,
}

/// The pipeline run summary (finalized/produced in Phase 7).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PipelineSummary {
    pub steps: Vec<StepResult>,
    pub successful: bool,
    pub halted: bool,
    #[serde(
        rename = "total_duration_ms",
        serialize_with = "duration_ms::serialize"
    )]
    pub total_duration: Duration,
    pub warnings: Vec<String>,
}

/// Adapts host `log`/`report-progress` callbacks into `PipelineEvent`s on the channel. Sends are
/// best-effort: a full or dropped receiver is ignored, never fatal.
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

/// One flattened, executable step (stages flattened in declaration order; §3.1 step 3).
/// `load_pipeline` (Task 4) constructs these and reads `stage` for the stage filter; the
/// Phase-7 runner reads the rest (dispatch, conditions, halt, step config) — dormant until then,
/// which trips clippy's dead-code lint under `-D warnings`.
pub(crate) struct FlatStep {
    pub(crate) stage: String,
    pub(crate) name: String,
    pub(crate) plugin: String,
    pub(crate) middleware: String,
    pub(crate) condition: Option<String>,
    pub(crate) halt_if: Option<String>,
    pub(crate) continue_on_error: bool,
    pub(crate) config: ConfigMap,
}

/// A loaded, ready-to-run pipeline. Opaque to external callers; the Phase-7 runner (same crate)
/// consumes the `pub(crate)` internals. Each plugin instance is kept alive for the whole run (§3.2).
pub struct Pipeline {
    pub(crate) plugins: IndexMap<String, PluginInstance>,
    pub(crate) steps: Vec<FlatStep>,
    pub(crate) acc: Accumulator,
    // Read by the Phase-7 runner (ReleaseContext).
    pub(crate) working_directory: PathBuf,
    // Populated by `load_pipeline`; read by the runner (during-execute timeout, Task 4).
    pub(crate) step_timeout: Option<Duration>,
    // `plugin_meta` stays write-only for now (a later feature reads it).
    #[allow(dead_code)]
    pub(crate) plugin_meta: IndexMap<String, PluginMetadata>,
}

impl Pipeline {
    /// Number of executable steps (after stage filtering).
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }
    /// Loaded plugin aliases, in declaration order.
    pub fn plugin_names(&self) -> Vec<&str> {
        self.plugins.keys().map(String::as_str).collect()
    }
}

mod runner;

pub(crate) use runner::run_pipeline;

#[cfg(test)]
mod serde_tests {
    use super::*;
    use crate::host::LogLevel;
    use std::time::Duration;

    #[test]
    fn step_log_serializes_tagged_with_lowercase_level() {
        let ev = PipelineEvent::StepLog {
            step: "s".into(),
            level: LogLevel::Warn,
            message: "m".into(),
        };
        let j = serde_json::to_string(&ev).unwrap();
        assert_eq!(
            j,
            r#"{"type":"step_log","step":"s","level":"warn","message":"m"}"#
        );
    }

    #[test]
    fn step_result_duration_serializes_as_milliseconds() {
        let sr = StepResult {
            name: "s".into(),
            successful: true,
            skipped: false,
            duration: Duration::from_millis(210),
            error_message: None,
            warnings: vec![],
        };
        let v = serde_json::to_value(&sr).unwrap();
        assert_eq!(v["duration_ms"], 210);
        assert!(v.get("duration").is_none());
    }

    #[test]
    fn summary_and_finished_event_are_tagged() {
        let summary = PipelineSummary {
            steps: vec![],
            successful: true,
            halted: false,
            total_duration: Duration::from_millis(12340),
            warnings: vec![],
        };
        let ev = PipelineEvent::PipelineFinished { summary };
        let v = serde_json::to_value(&ev).unwrap();
        assert_eq!(v["type"], "pipeline_finished");
        assert_eq!(v["summary"]["total_duration_ms"], 12340);
        assert_eq!(v["summary"]["successful"], true);
    }
}
