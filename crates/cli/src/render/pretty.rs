use std::collections::HashMap;

use super::summary::{build_table, fmt_duration};
use super::{Header, Renderer, error_block};
use crate::display::display_safe;
use console::style;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use moonlit_engine::logging::LogLevel;
use moonlit_engine::pipeline::{PipelineEvent, PipelineSummary, StepResult};

pub struct PrettyRenderer {
  mp: MultiProgress,
  plugins: HashMap<String, ProgressBar>,
  step: Option<ProgressBar>,
  verbose: bool,
}

impl PrettyRenderer {
  #[must_use]
  pub fn new(verbose: bool) -> Self {
    Self {
      mp: MultiProgress::new(),
      plugins: HashMap::new(),
      step: None,
      verbose,
    }
  }

  #[must_use]
  fn spinner_style() -> ProgressStyle {
    ProgressStyle::with_template("{spinner} {msg}")
      .unwrap()
      .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ")
  }

  fn print(&self, line: String) {
    let _ = self.mp.println(line);
  }

  fn log_text(&self, step: &str, level: LogLevel, message: &str) -> Option<String> {
    if matches!(level, LogLevel::Trace | LogLevel::Debug) && !self.verbose {
      return None;
    }
    let tag = match level {
      LogLevel::Trace => style("TRACE").dim(),
      LogLevel::Debug => style("DEBUG").blue(),
      LogLevel::Info => style("INFO").green(),
      LogLevel::Warn => style("WARN").yellow(),
      LogLevel::Error => style("ERROR").red(),
    };
    Some(format!("    {tag} {step}: {}", display_safe(message)))
  }

  fn log_line(&self, step: &str, level: LogLevel, message: &str) {
    if let Some(text) = self.log_text(step, level, message) {
      self.print(text);
    }
  }
}

impl Renderer for PrettyRenderer {
  fn header(&mut self, h: &Header) {
    self.print(format!("🌙 {}", style(format!("Moonlit v{}", h.version)).bold()));
    if let Some(name) = &h.name {
      self.print(format!("🚀 Executing release pipeline: {name}"));
    }
    self.print(format!("📁 Working directory: {}", h.working_dir.display()));
    let stages = if h.stages.is_empty() {
      "all".to_string()
    } else {
      h.stages.join(", ")
    };
    self.print(format!("⚙️  Configuration: {} (stages: {})", h.config_file, stages));
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
      PipelineEvent::PluginPullProgress { name, received, total } => {
        if let Some(pb) = self.plugins.get(name) {
          match total {
            Some(t) => pb.set_message(format!("pulling {name} {:.0}%", percent(*received, *t))),
            None => pb.set_message(format!("pulling {name} {received} bytes")),
          }
        }
      }
      PipelineEvent::PluginReady { name, version, cached } => {
        if let Some(pb) = self.plugins.remove(name) {
          let how = if *cached { "cached" } else { "pulled" };
          pb.finish_and_clear();
          self.print(ready_text(name, version, how));
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
        pb.set_message(format!("Step {}/{total} · {stage} › {name} ({run})", index + 1));
        self.step = Some(pb);
      }
      PipelineEvent::StepLog { step, level, message } => self.log_line(step, *level, message),
      PipelineEvent::StepProgress { step: _, message } => {
        if let Some(pb) = &self.step {
          pb.set_message(progress_text(message));
        }
      }
      PipelineEvent::StepSkipped { step, condition } => {
        if let Some(pb) = self.step.take() {
          pb.finish_and_clear();
        }
        self.print(format!("{} {step} (condition not met: {condition})", style("↷").dim()));
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
          self.print(release_failure(summary));
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
      self.print(warning_text(&result.name, w));
    }
    if result.successful {
      self.print(format!(
        "{} {} · {}",
        style("✔").green(),
        result.name,
        fmt_duration(result.duration)
      ));
    } else {
      self.print(step_failure(result));
    }
  }
}

#[must_use]
fn progress_text(message: &str) -> String {
  format!("… › {}", display_safe(message))
}

#[must_use]
fn ready_text(name: &str, version: &str, how: &str) -> String {
  format!("{} {name} {} ({how})", style("✔").green(), display_safe(version))
}

#[must_use]
fn warning_text(step: &str, warning: &str) -> String {
  format!("    {} {step}: {}", style("WARN").yellow(), display_safe(warning))
}

#[must_use]
fn step_failure(result: &StepResult) -> String {
  let err = error_block(result.error_message.as_deref().unwrap_or("failed"));
  format!(
    "{} {} · {} — {err}",
    style("✘").red(),
    result.name,
    fmt_duration(result.duration)
  )
}

#[must_use]
fn release_failure(summary: &PipelineSummary) -> String {
  let msg = summary
    .steps
    .iter()
    .rev()
    .find_map(|s| s.error_message.as_deref().map(error_block))
    .unwrap_or_else(|| "pipeline did not complete successfully".to_string());
  format!("{} Release failed: {msg}", style("❌").red())
}

#[expect(
  clippy::cast_precision_loss,
  reason = "a progress percentage does not need 64-bit precision"
)]
#[must_use]
fn percent(received: u64, total: u64) -> f64 {
  (received as f64 / total as f64) * 100.0
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::render::fixtures::{header, step, summary};

  fn started(name: &str) -> PipelineEvent {
    PipelineEvent::StepStarted {
      index: 1,
      total: 1,
      stage: "build".into(),
      name: name.into(),
      run: "tp.x".into(),
    }
  }

  fn finished(result: StepResult) -> PipelineEvent {
    PipelineEvent::StepFinished {
      step: result.name.clone(),
      result,
    }
  }

  #[test]
  fn plugin_spinner_lives_from_resolving_until_ready() {
    let mut r = PrettyRenderer::new(false);
    r.handle(&PipelineEvent::PluginResolving {
      name: "tp".into(),
      url: "file://x".into(),
    });
    assert!(r.plugins.contains_key("tp"));
    for total in [Some(10), None] {
      r.handle(&PipelineEvent::PluginPullProgress {
        name: "tp".into(),
        received: 5,
        total,
      });
    }
    r.handle(&PipelineEvent::PluginPullProgress {
      name: "unknown".into(),
      received: 1,
      total: None,
    });
    r.handle(&PipelineEvent::PluginReady {
      name: "tp".into(),
      version: "1.0.0".into(),
      cached: false,
    });
    assert!(r.plugins.is_empty());
    r.handle(&PipelineEvent::PluginReady {
      name: "never-resolved".into(),
      version: "1.0.0".into(),
      cached: true,
    });
    assert!(r.plugins.is_empty());
  }

  #[test]
  fn step_spinner_is_cleared_when_the_step_finishes() {
    let mut r = PrettyRenderer::new(true);
    r.header(&header(Some("demo"), &["build"]));
    r.handle(&started("s1"));
    assert!(r.step.is_some());
    r.handle(&PipelineEvent::StepProgress {
      step: "s1".into(),
      message: "halfway".into(),
    });
    for level in [
      LogLevel::Trace,
      LogLevel::Debug,
      LogLevel::Info,
      LogLevel::Warn,
      LogLevel::Error,
    ] {
      r.handle(&PipelineEvent::StepLog {
        step: "s1".into(),
        level,
        message: "m".into(),
      });
    }
    r.handle(&finished(step("s1", true, false, None, &["careful"])));
    assert!(r.step.is_none());

    r.handle(&started("s2"));
    r.handle(&finished(step("s2", false, false, None, &[])));
    assert!(r.step.is_none());
  }

  #[test]
  fn skipped_step_clears_the_spinner_and_ignores_its_finish() {
    let mut r = PrettyRenderer::new(false);
    r.header(&header(None, &[]));
    r.handle(&started("s1"));
    r.handle(&PipelineEvent::StepSkipped {
      step: "s1".into(),
      condition: "false".into(),
    });
    assert!(r.step.is_none());
    r.handle(&started("s2"));
    r.handle(&finished(step("s2", true, true, None, &[])));
    assert!(r.step.is_some());
    r.finish();
    assert!(r.step.is_none());
  }

  #[test]
  fn quiet_renderer_drops_debug_logs_and_reports_the_outcome() {
    let mut r = PrettyRenderer::new(false);
    r.handle(&PipelineEvent::StepLog {
      step: "s1".into(),
      level: LogLevel::Debug,
      message: "hidden".into(),
    });
    r.handle(&PipelineEvent::StepProgress {
      step: "s1".into(),
      message: "no spinner".into(),
    });
    r.handle(&PipelineEvent::StepSkipped {
      step: "s1".into(),
      condition: "false".into(),
    });
    r.handle(&PipelineEvent::PipelineHalted {
      after_step: "s1".into(),
      halt_if: "true".into(),
    });
    for (steps, successful) in [
      (vec![step("s1", true, false, None, &[])], true),
      (vec![step("s1", false, false, Some("boom"), &[])], false),
      (vec![], false),
    ] {
      r.handle(&PipelineEvent::PipelineFinished {
        summary: summary(steps, successful),
      });
    }
    r.finish();
    assert!(r.step.is_none());
  }

  #[test]
  fn multi_line_step_errors_keep_their_lines_and_escape_the_rest() {
    let error = "line1\nline2\r\u{1b}[2K\tend";
    let failed = step("s1", false, false, Some(error), &[]);
    let line = step_failure(&failed);
    assert!(!line.contains('\u{1b}') && !line.contains('\r'), "{line:?}");
    assert!(line.ends_with("line1\n    line2\\x0d\\x1b[2K\tend"), "{line:?}");
    let release = release_failure(&summary(vec![failed], false));
    assert!(release.ends_with("line1\n    line2\\x0d\\x1b[2K\tend"), "{release:?}");
  }

  #[test]
  fn plugin_text_with_escape_sequences_renders_escaped() {
    let r = PrettyRenderer::new(false);
    let texts = [
      r.log_text("s1", LogLevel::Info, "log\u{1b}[2K").expect("shown"),
      progress_text("progress\u{1b}[2K"),
      warning_text("s1", "warn\u{1b}[2K"),
      ready_text("tp", "1.0\u{1b}[2K", "cached"),
    ];
    for (text, expected) in texts.iter().zip(["log", "progress", "warn", "1.0"]) {
      assert!(!text.contains('\u{1b}'), "{text:?}");
      assert!(text.contains(&format!("{expected}\\x1b[2K")), "{text:?}");
    }
  }

  #[test]
  fn percent_is_a_ratio_of_received_over_total() {
    assert!((percent(25, 100) - 25.0).abs() < f64::EPSILON);
  }
}
