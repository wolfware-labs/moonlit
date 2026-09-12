//! Machine-readable rendering: one JSON object per event line (MVP_SPEC §9.4.6). The header is
//! a display concern and is intentionally NOT emitted — json is the pure engine event stream.

use std::io::Write;

use moonlit_engine::PipelineEvent;

use super::{Header, Renderer};

pub struct JsonRenderer<W: Write + Send> {
    out: W,
}

impl<W: Write + Send> JsonRenderer<W> {
    pub fn new(out: W) -> Self {
        Self { out }
    }
}

impl<W: Write + Send> Renderer for JsonRenderer<W> {
    fn header(&mut self, _header: &Header) {}

    fn handle(&mut self, event: &PipelineEvent) {
        if let Ok(line) = serde_json::to_string(event) {
            let _ = writeln!(self.out, "{line}");
        }
    }

    fn finish(&mut self) {
        let _ = self.out.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_engine::LogLevel;

    #[test]
    fn each_event_is_one_tagged_json_line() {
        let mut buf: Vec<u8> = Vec::new();
        {
            let mut r = JsonRenderer::new(&mut buf);
            r.header(&Header {
                version: "0.1.0",
                name: Some("p".into()),
                working_dir: "/w".into(),
                config_file: "release.yml".into(),
                stages: vec![],
            });
            r.handle(&PipelineEvent::StepLog {
                step: "s".into(),
                level: LogLevel::Info,
                message: "hi".into(),
            });
            r.finish();
        }
        let out = String::from_utf8(buf).unwrap();
        // Header emits nothing; exactly one event line.
        assert_eq!(out.lines().count(), 1, "{out}");
        assert_eq!(
            out.trim(),
            r#"{"type":"step_log","step":"s","level":"info","message":"hi"}"#
        );
    }
}
