use super::{Header, Renderer};
use moonlit_engine::pipeline::PipelineEvent;
use std::io::Write;

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
