use moonlit_plugin_sdk::prelude::*;
use std::collections::BTreeMap;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct RunModulesConfig {
    pub module_paths: Vec<String>,
    pub stages: Vec<String>,
    pub continue_on_module_error: bool,
    pub arguments: BTreeMap<String, String>,
}

/// Split a module path into `(-w dir, Option<-f basename>)`. A `.yml`/`.yaml`
/// path (case-insensitive) is a file: dir is its parent (or `.`), file is the
/// basename. Anything else is a directory: dir is the path, no `-f`.
// Not yet called from `execute` — wired into the spawn loop in Task 5.
#[allow(dead_code)]
fn split_path(path: &str) -> (&str, Option<&str>) {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".yml") || lower.ends_with(".yaml") {
        match path.rfind('/') {
            Some(i) => (&path[..i], Some(&path[i + 1..])),
            None => (".", Some(path)),
        }
    } else {
        (path, None)
    }
}

#[derive(Default)]
pub struct RunModules;

impl Middleware for RunModules {
    const NAME: &'static str = "run-modules";
    const DESCRIPTION: &'static str = "run nested Moonlit release files as modules";
    type Config = RunModulesConfig;

    fn execute(&self, _ctx: &Context, cfg: RunModulesConfig) -> MiddlewareResult {
        if cfg.module_paths.is_empty() {
            return MiddlewareResult::failure("No module paths provided for run-modules.");
        }
        // Spawn loop implemented in Task 5.
        MiddlewareResult::success()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_plugin_sdk::testing::{run, MockHost};

    #[test]
    fn directory_path_maps_to_working_dir_only() {
        assert_eq!(split_path("modules/foo"), ("modules/foo", None));
    }

    #[test]
    fn yml_path_splits_into_dir_and_basename() {
        assert_eq!(
            split_path("modules/foo/release.yml"),
            ("modules/foo", Some("release.yml"))
        );
        assert_eq!(split_path("a/b/c.yaml"), ("a/b", Some("c.yaml")));
    }

    #[test]
    fn yml_path_without_slash_uses_dot_dir() {
        assert_eq!(split_path("release.yml"), (".", Some("release.yml")));
    }

    #[test]
    fn yml_extension_match_is_case_insensitive() {
        assert_eq!(split_path("x/Deploy.YML"), ("x", Some("Deploy.YML")));
    }

    #[test]
    fn empty_module_paths_fails_before_any_spawn() {
        let host = MockHost::new();
        let ctx = Context::new(&host, "/w".into(), "modules".into());
        let cfg = RunModulesConfig::default(); // module_paths empty
        let r = run(&RunModules, &ctx, cfg);
        assert!(!r.is_success());
        assert_eq!(
            r.error_message(),
            Some("No module paths provided for run-modules.")
        );
        assert!(host.recorded_commands().is_empty());
    }
}
