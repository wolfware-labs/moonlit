use super::summary::{build_table, fmt_duration};
use super::{Header, Renderer, error_block, release_error};
use crate::display::display_safe;
use moonlit_engine::logging::LogLevel;
use moonlit_engine::pipeline::{PipelineEvent, StepResult};
use std::io::Write;

pub struct PlainRenderer<W: Write + Send> {
  out: W,
  verbose: bool,
}

impl<W: Write + Send> PlainRenderer<W> {
  #[must_use]
  pub fn new(out: W, verbose: bool) -> Self {
    Self { out, verbose }
  }
}

#[must_use]
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
    let _ = writeln!(self.out, "Working directory: {}", h.working_dir.display());
    let stages = if h.stages.is_empty() {
      "all".to_string()
    } else {
      h.stages.join(", ")
    };
    let _ = writeln!(self.out, "Configuration: {} (stages: {})", h.config_file, stages);
  }

  fn handle(&mut self, event: &PipelineEvent) {
    match event {
      PipelineEvent::PluginResolving { name, .. } => {
        let _ = writeln!(self.out, "resolving {}", display_safe(name));
      }
      PipelineEvent::PluginPullProgress { .. } => {}
      PipelineEvent::PluginReady { name, version, cached } => {
        let how = if *cached { "cached" } else { "pulled" };
        let _ = writeln!(self.out, "ready {} {} ({how})", display_safe(name), display_safe(version));
      }
      PipelineEvent::StepStarted {
        index,
        total,
        stage,
        name,
        run,
      } => {
        let _ = writeln!(self.out, "Step {}/{total} · {stage} › {name} ({run})", index + 1);
      }
      PipelineEvent::StepLog { step, level, message } => {
        if matches!(level, LogLevel::Trace | LogLevel::Debug) && !self.verbose {
          return; // DEBUG/TRACE only with -v (§9.4.7)
        }
        let _ = writeln!(self.out, "  [{}] {step}: {}", level_tag(*level), display_safe(message));
      }
      PipelineEvent::StepProgress { step, message } => {
        let _ = writeln!(self.out, "  {step}: {}", display_safe(message));
      }
      PipelineEvent::StepSkipped { step, condition } => {
        let _ = writeln!(self.out, "↷ {step} (condition not met: {condition})");
      }
      PipelineEvent::StepFinished { result, .. } => self.step_finished(result),
      PipelineEvent::PipelineHalted { after_step, halt_if } => {
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
          let _ = writeln!(self.out, "Release failed: {}", release_error(summary));
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
      let _ = writeln!(self.out, "  [WARN] {}: {}", result.name, display_safe(w));
    }
    if result.successful {
      let _ = writeln!(self.out, "✔ {} · {}", result.name, fmt_duration(result.duration));
    } else {
      let err = error_block(result.error_message.as_deref().unwrap_or("failed"));
      let _ = writeln!(self.out, "✘ {} · {} — {err}", result.name, fmt_duration(result.duration));
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::render::fixtures::{header, step, summary};

  fn render(verbose: bool, events: &[PipelineEvent]) -> String {
    let mut out = Vec::new();
    {
      let mut r = PlainRenderer::new(&mut out, verbose);
      for event in events {
        r.handle(event);
      }
      r.finish();
    }
    String::from_utf8(out).unwrap()
  }

  fn log(level: LogLevel) -> PipelineEvent {
    PipelineEvent::StepLog {
      step: "s1".into(),
      level,
      message: "hello".into(),
    }
  }

  fn finished(result: StepResult) -> PipelineEvent {
    PipelineEvent::StepFinished {
      step: result.name.clone(),
      result,
    }
  }

  #[test]
  fn failed_step_message_from_a_plugin_renders_escaped() {
    let error = "blocked from running 'git' (optional permission: \"Tags.\u{1b}[1A\u{1b}[2K\")";
    let text = render(
      false,
      &[
        finished(step("s1", false, false, Some(error), &[])),
        PipelineEvent::PipelineFinished {
          summary: summary(vec![step("s1", false, false, Some(error), &[])], false),
        },
      ],
    );
    assert!(!text.contains('\u{1b}'), "{text:?}");
    assert!(text.contains("\"Tags.\\x1b[1A\\x1b[2K\")"), "{text}");
  }

  #[test]
  fn multi_line_step_errors_keep_their_lines_and_escape_the_rest() {
    let error = "line1\nline2\r\u{1b}[2K\tend";
    let text = render(
      false,
      &[
        finished(step("s1", false, false, Some(error), &[])),
        PipelineEvent::PipelineFinished {
          summary: summary(vec![step("s1", false, false, Some(error), &[])], false),
        },
      ],
    );
    assert!(!text.contains('\u{1b}') && !text.contains('\r'), "{text:?}");
    assert!(
      text.contains("✘ s1 · 1.5s — line1\n    line2\\x0d\\x1b[2K\tend\n"),
      "{text:?}"
    );
    assert!(
      text.contains("Release failed: line1\n    line2\\x0d\\x1b[2K\tend\n"),
      "{text:?}"
    );
  }

  #[test]
  fn plugin_text_with_escape_sequences_renders_escaped() {
    let text = render(
      false,
      &[
        PipelineEvent::PluginResolving {
          name: "rp\u{1b}[8m".into(),
          url: "oci://h/o/rp:1.0.0".into(),
        },
        PipelineEvent::PluginReady {
          name: "tp\u{1b}[8m".into(),
          version: "1.0\u{1b}[2K".into(),
          cached: true,
        },
        PipelineEvent::StepLog {
          step: "s1".into(),
          level: LogLevel::Info,
          message: "log\u{1b}[2K".into(),
        },
        PipelineEvent::StepProgress {
          step: "s1".into(),
          message: "progress\u{1b}[2K".into(),
        },
        finished(step("s1", true, false, None, &["warn\u{1b}[2K"])),
      ],
    );
    assert!(!text.contains('\u{1b}'), "{text:?}");
    for expected in [
      "resolving rp\\x1b[8m",
      "ready tp\\x1b[8m 1.0\\x1b[2K",
      "log\\x1b[2K",
      "progress\\x1b[2K",
      "warn\\x1b[2K",
    ] {
      assert!(text.contains(expected), "missing {expected:?} in {text:?}");
    }
  }

  #[test]
  fn header_lists_name_directory_and_stages() {
    let mut out = Vec::new();
    PlainRenderer::new(&mut out, false).header(&header(Some("demo"), &["build", "test"]));
    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("Executing release pipeline: demo"));
    assert!(text.contains("Configuration: release.yml (stages: build, test)"));
  }

  #[test]
  fn header_without_name_or_stages_says_all() {
    let mut out = Vec::new();
    PlainRenderer::new(&mut out, false).header(&header(None, &[]));
    let text = String::from_utf8(out).unwrap();
    assert!(!text.contains("Executing release pipeline"));
    assert!(text.contains("(stages: all)"));
  }

  #[test]
  fn debug_and_trace_logs_need_verbose() {
    let quiet = render(false, &[log(LogLevel::Trace), log(LogLevel::Debug), log(LogLevel::Info)]);
    assert!(!quiet.contains("[TRACE]") && !quiet.contains("[DEBUG]"));
    assert!(quiet.contains("  [INFO] s1: hello"));

    let loud = render(
      true,
      &[
        log(LogLevel::Trace),
        log(LogLevel::Debug),
        log(LogLevel::Warn),
        log(LogLevel::Error),
      ],
    );
    for tag in ["[TRACE]", "[DEBUG]", "[WARN]", "[ERROR]"] {
      assert!(loud.contains(tag), "missing {tag} in {loud}");
    }
  }

  #[test]
  fn plugin_and_step_events_are_one_line_each() {
    let text = render(
      false,
      &[
        PipelineEvent::PluginResolving {
          name: "tp".into(),
          url: "file://x".into(),
        },
        PipelineEvent::PluginPullProgress {
          name: "tp".into(),
          received: 1,
          total: Some(2),
        },
        PipelineEvent::PluginReady {
          name: "tp".into(),
          version: "1.0.0".into(),
          cached: true,
        },
        PipelineEvent::PluginReady {
          name: "tq".into(),
          version: "2.0.0".into(),
          cached: false,
        },
        PipelineEvent::StepStarted {
          index: 0,
          total: 2,
          stage: "build".into(),
          name: "s1".into(),
          run: "tp.x".into(),
        },
        PipelineEvent::StepProgress {
          step: "s1".into(),
          message: "halfway".into(),
        },
        PipelineEvent::StepSkipped {
          step: "s2".into(),
          condition: "false".into(),
        },
        PipelineEvent::PipelineHalted {
          after_step: "s1".into(),
          halt_if: "true".into(),
        },
      ],
    );
    for expected in [
      "resolving tp",
      "ready tp 1.0.0 (cached)",
      "ready tq 2.0.0 (pulled)",
      "Step 1/2 · build › s1 (tp.x)",
      "  s1: halfway",
      "↷ s2 (condition not met: false)",
      "halted after s1 (haltIf: true)",
    ] {
      assert!(text.contains(expected), "missing {expected:?} in {text}");
    }
    assert_eq!(text.lines().count(), 7);
  }

  #[test]
  fn finished_steps_report_warnings_success_and_failure() {
    let text = render(
      false,
      &[
        finished(step("ok", true, false, None, &["careful"])),
        finished(step("bad", false, false, Some("boom"), &[])),
        finished(step("mute", false, false, None, &[])),
        finished(step("hidden", true, true, None, &[])),
      ],
    );
    assert!(text.contains("  [WARN] ok: careful"));
    assert!(text.contains("✔ ok · 1.5s"));
    assert!(text.contains("✘ bad · 1.5s — boom"));
    assert!(text.contains("✘ mute · 1.5s — failed"));
    assert!(!text.contains("hidden"));
  }

  #[test]
  fn pipeline_finished_reports_success_or_the_last_error() {
    let finished_pipeline = |steps, successful| PipelineEvent::PipelineFinished {
      summary: summary(steps, successful),
    };
    let ok = render(false, &[finished_pipeline(vec![step("s1", true, false, None, &[])], true)]);
    assert!(ok.contains("Release completed in 2.50 seconds"));

    let failed = render(
      false,
      &[finished_pipeline(
        vec![
          step("s1", false, false, Some("first"), &[]),
          step("s2", false, false, Some("last"), &[]),
        ],
        false,
      )],
    );
    assert!(failed.contains("Release failed: last"));

    let unexplained = render(false, &[finished_pipeline(vec![], false)]);
    assert!(unexplained.contains("Release failed: pipeline did not complete successfully"));
  }
}
