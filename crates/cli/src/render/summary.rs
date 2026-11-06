use std::time::Duration;

use comfy_table::{ContentArrangement, Table, presets::UTF8_BORDERS_ONLY};
use moonlit_engine::pipeline::{PipelineSummary, StepResult};

use crate::display::display_safe_text;

#[must_use]
pub fn fmt_duration(d: Duration) -> String {
  let ms = d.as_millis();
  if ms < 1000 {
    format!("{ms}ms")
  } else {
    format!("{:.1}s", d.as_secs_f64())
  }
}

#[must_use]
fn status(step: &StepResult) -> &'static str {
  if step.skipped {
    "SKIPPED"
  } else if step.successful {
    "SUCCESS"
  } else {
    "FAILED"
  }
}

#[must_use]
pub fn build_table(summary: &PipelineSummary) -> Table {
  let mut table = Table::new();
  table
    .load_style(UTF8_BORDERS_ONLY)
    .set_content_arrangement(ContentArrangement::Dynamic)
    .set_header(["Step", "Status", "Duration", "Error"]);
  for step in &summary.steps {
    let duration = if step.skipped {
      "-".to_string()
    } else {
      fmt_duration(step.duration)
    };
    let error = step
      .error_message
      .as_deref()
      .map_or_else(|| "-".to_string(), display_safe_text);
    table.add_row([&step.name, status(step), &duration, &error]);
  }
  table
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::render::fixtures::{step, summary};

  #[test]
  fn durations_under_a_second_are_milliseconds() {
    assert_eq!(fmt_duration(Duration::from_millis(250)), "250ms");
  }

  #[test]
  fn durations_of_a_second_or_more_are_seconds() {
    assert_eq!(fmt_duration(Duration::from_millis(1500)), "1.5s");
  }

  #[test]
  fn table_lists_each_status_and_error() {
    let table = build_table(&summary(
      vec![
        step("ok", true, false, None, &[]),
        step("bad", false, false, Some("boom"), &[]),
        step("skip", true, true, None, &[]),
      ],
      false,
    ))
    .to_string();
    assert!(table.contains("SUCCESS"));
    assert!(table.contains("FAILED"));
    assert!(table.contains("SKIPPED"));
    assert!(table.contains("boom"));
    assert!(table.contains("1.5s"));
  }

  #[test]
  fn multi_line_errors_stay_on_separate_lines_and_escape_the_rest() {
    let table = build_table(&summary(
      vec![step("bad", false, false, Some("line1\nline2\u{1b}[2K"), &[])],
      false,
    ))
    .to_string();
    assert!(!table.contains('\u{1b}'), "{table:?}");
    assert!(table.contains("line1") && table.contains("line2\\x1b[2K"), "{table}");
    let rows: Vec<&str> = table.lines().filter(|l| l.contains("line1") || l.contains("line2")).collect();
    assert_eq!(rows.len(), 2, "{table}");
  }
}
