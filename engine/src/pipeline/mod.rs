//! Shared pipeline data types (the run loop lands here in Phase 7).

use std::time::Duration;

use indexmap::IndexMap;
use tokio::sync::mpsc::Sender;

use crate::config::model::ConfigMap;
use crate::expr::Accumulator;
use crate::host::{HostEventSink, LogLevel, PluginInstance, PluginMetadata};

/// Events streamed to the CLI over an mpsc channel. Phase 6 emits the plugin-load events (and
/// `StepLog`/`StepProgress` while a plugin's `init` runs); the `Step*`/halt/finish events are
/// produced by the Phase-7 runner.
#[derive(Clone, Debug, PartialEq)]
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
#[derive(Clone, Debug, PartialEq)]
pub struct StepResult {
    pub name: String,
    pub successful: bool,
    pub skipped: bool,
    pub duration: Duration,
    pub error_message: Option<String>,
    pub warnings: Vec<String>,
}

/// The pipeline run summary (finalized/produced in Phase 7).
#[derive(Clone, Debug, PartialEq)]
pub struct PipelineSummary {
    pub steps: Vec<StepResult>,
    pub successful: bool,
    pub halted: bool,
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
// TODO(Task 4): remove once `load_pipeline` constructs `FlatStep` and the Phase-7 runner reads
// these fields; both are dormant until then, which trips clippy's dead-code lint under `-D warnings`.
#[allow(dead_code)]
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
    // TODO(Task 4/Phase 7): remove once `load_pipeline` populates `acc` from layered config and
    // the Phase-7 runner reads it during substitution; dormant until then, which trips clippy's
    // dead-code lint under `-D warnings`.
    #[allow(dead_code)]
    pub(crate) acc: Accumulator,
    // TODO(Task 4/Phase 7): remove once `load_pipeline` populates `plugin_meta` and the
    // Phase-7 runner reads it (e.g. for step logging); dormant until then, which trips clippy's
    // dead-code lint under `-D warnings`.
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
