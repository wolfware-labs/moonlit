//! Final normalization before validation. Missing collections are already coalesced to empty at
//! the `convert` boundary, so the meaningful step here is trimming `name`.

use crate::config::model::PipelineConfig;

/// Trim the pipeline `name` (§4.2).
pub fn cleanup(mut config: PipelineConfig) -> PipelineConfig {
    config.name = config.name.trim().to_string();
    config
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::{PipelineConfig, Span, Spanned};
    use indexmap::IndexMap;

    fn blank() -> PipelineConfig {
        PipelineConfig {
            name: "  demo  ".to_string(),
            arguments: IndexMap::new(),
            variables: IndexMap::new(),
            plugins: Spanned::new(Vec::new(), Span::point(0)),
            stages: Spanned::new(Vec::new(), Span::point(0)),
        }
    }

    #[test]
    fn trims_name() {
        assert_eq!(cleanup(blank()).name, "demo");
    }
}
