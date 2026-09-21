use std::time::Duration;

use comfy_table::{ContentArrangement, Table, presets::UTF8_BORDERS_ONLY};
use moonlit_engine::pipeline::{PipelineSummary, StepResult};

pub fn fmt_duration(d: Duration) -> String {
    let ms = d.as_millis();
    if ms < 1000 {
        format!("{ms}ms")
    } else {
        format!("{:.1}s", d.as_secs_f64())
    }
}

fn status(step: &StepResult) -> &'static str {
    if step.skipped {
        "SKIPPED"
    } else if step.successful {
        "SUCCESS"
    } else {
        "FAILED"
    }
}

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
            .clone()
            .unwrap_or_else(|| "-".to_string());
        table.add_row([&step.name, status(step), &duration, &error]);
    }
    table
}
