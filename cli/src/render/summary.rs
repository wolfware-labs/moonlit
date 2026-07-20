//! The execution summary table (MVP_SPEC §9.4.4) and duration formatting, shared by the
//! plain and pretty renderers.

use std::time::Duration;

use comfy_table::{ContentArrangement, Table, presets::UTF8_BORDERS_ONLY};
use moonlit_engine::{PipelineSummary, StepResult};

/// `210ms` under a second, else `3.1s`.
// Not yet called from `main`: consumed by the plain/pretty renderers once the run command is
// wired in a later task.
#[allow(dead_code)]
pub fn fmt_duration(d: Duration) -> String {
    let ms = d.as_millis();
    if ms < 1000 {
        format!("{ms}ms")
    } else {
        format!("{:.1}s", d.as_secs_f64())
    }
}

#[allow(dead_code)]
fn status(step: &StepResult) -> &'static str {
    if step.skipped {
        "SKIPPED"
    } else if step.successful {
        "SUCCESS"
    } else {
        "FAILED"
    }
}

/// Build the `Execution Summary` table: Step · Status · Duration · Error.
// Not yet called from `main`: consumed by the plain/pretty renderers once the run command is
// wired in a later task.
#[allow(dead_code)]
pub fn build_table(summary: &PipelineSummary) -> Table {
    let mut table = Table::new();
    table
        .load_preset(UTF8_BORDERS_ONLY)
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
            .clone()
            .unwrap_or_else(|| "-".to_string());
        table.add_row([&step.name, status(step), &duration, &error]);
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(name: &str, ok: bool, skipped: bool, ms: u64, err: Option<&str>) -> StepResult {
        StepResult {
            name: name.into(),
            successful: ok,
            skipped,
            duration: Duration::from_millis(ms),
            error_message: err.map(str::to_string),
            warnings: vec![],
        }
    }

    #[test]
    fn duration_formats_ms_then_seconds() {
        assert_eq!(fmt_duration(Duration::from_millis(210)), "210ms");
        assert_eq!(fmt_duration(Duration::from_millis(3100)), "3.1s");
    }

    #[test]
    fn table_renders_each_status_and_dash_for_skipped() {
        let summary = PipelineSummary {
            steps: vec![
                step("a", true, false, 210, None),
                step("b", false, false, 3100, Some("boom")),
                step("c", false, true, 0, None),
            ],
            successful: false,
            halted: false,
            total_duration: Duration::from_millis(3310),
            warnings: vec![],
        };
        let out = build_table(&summary).to_string();
        assert!(out.contains("SUCCESS"), "{out}");
        assert!(out.contains("FAILED"), "{out}");
        assert!(out.contains("SKIPPED"), "{out}");
        assert!(out.contains("210ms"), "{out}");
        assert!(out.contains("boom"), "{out}");
        // Skipped row shows '-' for duration.
        let c_line = out.lines().find(|l| l.contains(" c ")).unwrap_or("");
        assert!(
            c_line.contains('-'),
            "skipped duration should be '-': {c_line}"
        );
    }
}
