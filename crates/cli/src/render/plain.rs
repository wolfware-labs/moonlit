use super::summary::{build_table, fmt_duration};
use super::{Header, Renderer};
use moonlit_engine::logging::LogLevel;
use moonlit_engine::pipeline::{PipelineEvent, StepResult};
use std::io::Write;

pub struct PlainRenderer<W: Write + Send> {
    out: W,
    verbose: bool,
}

impl<W: Write + Send> PlainRenderer<W> {
    pub fn new(out: W, verbose: bool) -> Self {
        Self { out, verbose }
    }
}

fn level_tag(level: LogLevel) -> &'static str {
    match level {
        LogLevel::Trace => "TRACE",
        LogLevel::Debug => "DEBUG",
        LogLevel::Info => "INFO",
        LogLevel::Warn => "WARN",
        LogLevel::Error => "ERROR",
    }
}

impl<W: Write + Send> Renderer for PlainRenderer<W> {
    fn header(&mut self, h: &Header) {
        let _ = writeln!(self.out, "Moonlit v{}", h.version);
        if let Some(name) = &h.name {
            let _ = writeln!(self.out, "Executing release pipeline: {name}");
        }
        let _ = writeln!(self.out, "Working directory: {}", h.working_dir);
        let stages = if h.stages.is_empty() {
            "all".to_string()
        } else {
            h.stages.join(", ")
        };
        let _ = writeln!(
            self.out,
            "Configuration: {} (stages: {})",
            h.config_file, stages
        );
    }

    fn handle(&mut self, event: &PipelineEvent) {
        match event {
            PipelineEvent::PluginResolving { name, .. } => {
                let _ = writeln!(self.out, "resolving {name}");
            }
            PipelineEvent::PluginPullProgress { .. } => {}
            PipelineEvent::PluginReady {
                name,
                version,
                cached,
            } => {
                let how = if *cached { "cached" } else { "pulled" };
                let _ = writeln!(self.out, "ready {name} {version} ({how})");
            }
            PipelineEvent::StepStarted {
                index,
                total,
                stage,
                name,
                run,
            } => {
                let _ = writeln!(self.out, "Step {index}/{total} · {stage} › {name} ({run})");
            }
            PipelineEvent::StepLog {
                step,
                level,
                message,
            } => {
                if matches!(level, LogLevel::Trace | LogLevel::Debug) && !self.verbose {
                    return; // DEBUG/TRACE only with -v (§9.4.7)
                }
                let _ = writeln!(self.out, "  [{}] {step}: {message}", level_tag(*level));
            }
            PipelineEvent::StepProgress { step, message } => {
                let _ = writeln!(self.out, "  {step}: {message}");
            }
            PipelineEvent::StepSkipped { step, condition } => {
                let _ = writeln!(self.out, "↷ {step} (condition not met: {condition})");
            }
            PipelineEvent::StepFinished { result, .. } => self.step_finished(result),
            PipelineEvent::PipelineHalted {
                after_step,
                halt_if,
            } => {
                let _ = writeln!(self.out, "halted after {after_step} (haltIf: {halt_if})");
            }
            PipelineEvent::PipelineFinished { summary } => {
                let _ = writeln!(self.out, "{}", build_table(summary));
                if summary.successful {
                    let _ = writeln!(
                        self.out,
                        "Release completed in {:.2} seconds",
                        summary.total_duration.as_secs_f64()
                    );
                } else {
                    let msg = summary
                        .steps
                        .iter()
                        .rev()
                        .find_map(|s| s.error_message.clone())
                        .unwrap_or_else(|| "pipeline did not complete successfully".to_string());
                    let _ = writeln!(self.out, "Release failed: {msg}");
                }
            }
        }
    }

    fn finish(&mut self) {
        let _ = self.out.flush();
    }
}

impl<W: Write + Send> PlainRenderer<W> {
    fn step_finished(&mut self, result: &StepResult) {
        if result.skipped {
            return; // the `↷` line was already printed on StepSkipped
        }
        for w in &result.warnings {
            let _ = writeln!(self.out, "  [WARN] {}: {w}", result.name);
        }
        if result.successful {
            let _ = writeln!(
                self.out,
                "✔ {} · {}",
                result.name,
                fmt_duration(result.duration)
            );
        } else {
            let err = result.error_message.as_deref().unwrap_or("failed");
            let _ = writeln!(
                self.out,
                "✘ {} · {} — {err}",
                result.name,
                fmt_duration(result.duration)
            );
        }
    }
}
