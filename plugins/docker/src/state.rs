//! Plugin-wide shared state (one instance per pipeline run): the buildx builder
//! name recorded by `setup-buildx` and read by `build-and-push`.

use moonlit_plugin_sdk::prelude::*;

/// Shared state for the docker plugin.
#[derive(Default)]
pub struct DockerShared {
    /// The buildx builder name, once `setup-buildx` has created one.
    pub builder: Shared<Option<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_defaults_none_and_round_trips() {
        let s = DockerShared::default();
        assert_eq!(s.builder.get(), None);
        s.builder.set(Some("moonlit-builder-x".to_string()));
        assert_eq!(s.builder.get(), Some("moonlit-builder-x".to_string()));
    }
}
