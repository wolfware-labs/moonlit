//! Plain, ANSI-free rendering for CI / non-TTY (MVP_SPEC §9.4.6).

use std::io::Write;

use moonlit_engine::{LogLevel, PipelineEvent, StepResult};

use super::summary::{build_table, fmt_duration};
use super::{Header, Renderer};

// Not yet constructed: the run command instantiates a renderer via `render::for_mode` in a
// later task.
#[allow(dead_code)]
pub struct PlainRenderer<W: Write + Send> {
    out: W,
    verbose: bool,
}

impl<W: Write + Send> PlainRenderer<W> {
    #[allow(dead_code)]
    pub fn new(out: W, verbose: bool) -> Self {
        Self { out, verbose }
    }
}

#[allow(dead_code)]
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
    #[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_engine::{PipelineSummary, StepResult};
    use std::time::Duration;

    fn render(events: &[PipelineEvent]) -> String {
        let mut buf: Vec<u8> = Vec::new();
        {
            let mut r = PlainRenderer::new(&mut buf, false);
            for e in events {
                r.handle(e);
            }
            r.finish();
        }
        String::from_utf8(buf).unwrap()
    }

    fn render_one(event: &PipelineEvent, verbose: bool) -> String {
        let mut buf: Vec<u8> = Vec::new();
        {
            let mut r = PlainRenderer::new(&mut buf, verbose);
            r.handle(event);
        }
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn debug_logs_hidden_without_verbose() {
        let ev = PipelineEvent::StepLog {
            step: "s".into(),
            level: LogLevel::Debug,
            message: "d".into(),
        };
        assert!(render_one(&ev, false).is_empty());
        assert!(render_one(&ev, true).contains("DEBUG"));
    }

    #[test]
    fn skip_prints_condition_line_and_suppresses_step_finished() {
        let out = render(&[
            PipelineEvent::StepSkipped {
                step: "commits".into(),
                condition: "output.x == 0".into(),
            },
            PipelineEvent::StepFinished {
                step: "commits".into(),
                result: StepResult {
                    name: "commits".into(),
                    successful: true,
                    skipped: true,
                    duration: Duration::ZERO,
                    error_message: None,
                    warnings: vec![],
                },
            },
        ]);
        assert!(
            out.contains("↷ commits (condition not met: output.x == 0)"),
            "{out}"
        );
        assert!(
            !out.contains('✔'),
            "skipped step must not print a ✔ line: {out}"
        );
    }

    #[test]
    fn failed_step_and_footer() {
        let summary = PipelineSummary {
            steps: vec![StepResult {
                name: "push".into(),
                successful: false,
                skipped: false,
                duration: Duration::from_millis(3100),
                error_message: Some("boom".into()),
                warnings: vec![],
            }],
            successful: false,
            halted: false,
            total_duration: Duration::from_millis(3100),
            warnings: vec![],
        };
        let out = render(&[
            PipelineEvent::StepFinished {
                step: "push".into(),
                result: summary.steps[0].clone(),
            },
            PipelineEvent::PipelineFinished { summary },
        ]);
        assert!(out.contains("✘ push · 3.1s — boom"), "{out}");
        assert!(out.contains("Release failed: boom"), "{out}");
    }

    #[test]
    fn success_footer_reports_seconds() {
        let summary = PipelineSummary {
            steps: vec![],
            successful: true,
            halted: false,
            total_duration: Duration::from_millis(12340),
            warnings: vec![],
        };
        let out = render(&[PipelineEvent::PipelineFinished { summary }]);
        assert!(out.contains("Release completed in 12.34 seconds"), "{out}");
    }
}
