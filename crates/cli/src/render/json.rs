use super::{Header, Renderer};
use moonlit_engine::pipeline::PipelineEvent;
use std::io::Write;

pub struct JsonRenderer<W: Write + Send> {
  out: W,
}

impl<W: Write + Send> JsonRenderer<W> {
  #[must_use]
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
  use crate::render::fixtures::header;

  #[test]
  fn writes_one_json_line_per_event_and_nothing_for_the_header() {
    let mut out = Vec::new();
    {
      let mut r = JsonRenderer::new(&mut out);
      r.header(&header(None, &[]));
      r.handle(&PipelineEvent::PipelineHalted {
        after_step: "s1".into(),
        halt_if: "x".into(),
      });
      r.finish();
    }
    let text = String::from_utf8(out).unwrap();
    assert_eq!(text.lines().count(), 1);
    assert!(text.contains(r#""type":"pipeline_halted""#));
  }
}
