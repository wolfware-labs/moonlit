//! Rich TTY rendering (MVP_SPEC §9.4.2–§9.4.4): indicatif progress for plugin resolution, a
//! step spinner with an indented live log region, and a comfy-table summary. Output goes to
//! stderr so json/stdout stays machine-clean; here everything is human output.

use std::collections::HashMap;

use console::style;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use moonlit_engine::{LogLevel, PipelineEvent, StepResult};

use super::summary::{build_table, fmt_duration};
use super::{Header, Renderer};

pub struct PrettyRenderer {
    mp: MultiProgress,
    /// Resolution bars, keyed by plugin name.
    plugins: HashMap<String, ProgressBar>,
    /// The active step spinner.
    step: Option<ProgressBar>,
    verbose: bool,
}

impl PrettyRenderer {
    pub fn new(verbose: bool) -> Self {
        Self {
            mp: MultiProgress::new(),
            plugins: HashMap::new(),
            step: None,
            verbose,
        }
    }

    fn spinner_style() -> ProgressStyle {
        ProgressStyle::with_template("{spinner} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ")
    }

    fn print(&self, line: String) {
        // Route through MultiProgress so lines don't corrupt active bars.
        let _ = self.mp.println(line);
    }

    fn log_line(&self, step: &str, level: LogLevel, message: &str) {
        if matches!(level, LogLevel::Trace | LogLevel::Debug) && !self.verbose {
            return;
        }
        let tag = match level {
            LogLevel::Trace => style("TRACE").dim(),
            LogLevel::Debug => style("DEBUG").blue(),
            LogLevel::Info => style("INFO").green(),
            LogLevel::Warn => style("WARN").yellow(),
            LogLevel::Error => style("ERROR").red(),
        };
        self.print(format!("    {tag} {step}: {message}"));
    }
}

impl Renderer for PrettyRenderer {
    fn header(&mut self, h: &Header) {
        self.print(format!(
            "🌙 {}",
            style(format!("Moonlit v{}", h.version)).bold()
        ));
        if let Some(name) = &h.name {
            self.print(format!("🚀 Executing release pipeline: {name}"));
        }
        self.print(format!("📁 Working directory: {}", h.working_dir));
        let stages = if h.stages.is_empty() {
            "all".to_string()
        } else {
            h.stages.join(", ")
        };
        self.print(format!(
            "⚙️  Configuration: {} (stages: {})",
            h.config_file, stages
        ));
    }

    fn handle(&mut self, event: &PipelineEvent) {
        match event {
            PipelineEvent::PluginResolving { name, .. } => {
                let pb = self.mp.add(ProgressBar::new_spinner());
                pb.set_style(Self::spinner_style());
                pb.set_message(format!("resolving {name}"));
                pb.enable_steady_tick(std::time::Duration::from_millis(100));
                self.plugins.insert(name.clone(), pb);
            }
            PipelineEvent::PluginPullProgress {
                name,
                received,
                total,
            } => {
                if let Some(pb) = self.plugins.get(name) {
                    match total {
                        Some(t) => pb.set_message(format!(
                            "pulling {name} {:.0}%",
                            (*received as f64 / *t as f64) * 100.0
                        )),
                        None => pb.set_message(format!("pulling {name} {received} bytes")),
                    }
                }
            }
            PipelineEvent::PluginReady {
                name,
                version,
                cached,
            } => {
                if let Some(pb) = self.plugins.remove(name) {
                    let how = if *cached { "cached" } else { "pulled" };
                    pb.finish_and_clear();
                    self.print(format!("{} {name} {version} ({how})", style("✔").green()));
                }
            }
            PipelineEvent::StepStarted {
                index,
                total,
                stage,
                name,
                run,
            } => {
                let pb = self.mp.add(ProgressBar::new_spinner());
                pb.set_style(Self::spinner_style());
                pb.enable_steady_tick(std::time::Duration::from_millis(100));
                pb.set_message(format!("Step {index}/{total} · {stage} › {name} ({run})"));
                self.step = Some(pb);
            }
            PipelineEvent::StepLog {
                step,
                level,
                message,
            } => self.log_line(step, *level, message),
            PipelineEvent::StepProgress { step: _, message } => {
                if let Some(pb) = &self.step {
                    pb.set_message(format!("… › {message}"));
                }
            }
            PipelineEvent::StepSkipped { step, condition } => {
                if let Some(pb) = self.step.take() {
                    pb.finish_and_clear();
                }
                self.print(format!(
                    "{} {step} (condition not met: {condition})",
                    style("↷").dim()
                ));
            }
            PipelineEvent::StepFinished { result, .. } => self.step_finished(result),
            PipelineEvent::PipelineHalted { after_step, .. } => {
                self.print(format!("⏹ halted after {after_step}"));
            }
            PipelineEvent::PipelineFinished { summary } => {
                self.print(format!("\n{}", style("Execution Summary").bold()));
                self.print(build_table(summary).to_string());
                if summary.successful {
                    self.print(format!(
                        "{} Release completed in {:.2} seconds",
                        style("✅").green(),
                        summary.total_duration.as_secs_f64()
                    ));
                } else {
                    let msg = summary
                        .steps
                        .iter()
                        .rev()
                        .find_map(|s| s.error_message.clone())
                        .unwrap_or_else(|| "pipeline did not complete successfully".to_string());
                    self.print(format!("{} Release failed: {msg}", style("❌").red()));
                }
            }
        }
    }

    fn finish(&mut self) {
        if let Some(pb) = self.step.take() {
            pb.finish_and_clear();
        }
    }
}

impl PrettyRenderer {
    fn step_finished(&mut self, result: &StepResult) {
        if result.skipped {
            return;
        }
        if let Some(pb) = self.step.take() {
            pb.finish_and_clear();
        }
        for w in &result.warnings {
            self.print(format!(
                "    {} {}: {w}",
                style("WARN").yellow(),
                result.name
            ));
        }
        if result.successful {
            self.print(format!(
                "{} {} · {}",
                style("✔").green(),
                result.name,
                fmt_duration(result.duration)
            ));
        } else {
            let err = result.error_message.as_deref().unwrap_or("failed");
            self.print(format!(
                "{} {} · {} — {err}",
                style("✘").red(),
                result.name,
                fmt_duration(result.duration)
            ));
        }
    }
}
