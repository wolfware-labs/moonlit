//! `generate-changelog` — produce structured changelog categories. The optional AI
//! refinement path is DEFERRED: setting either AI flag fails fast.

use moonlit_plugin_sdk::prelude::*;

use crate::changelog::ChangelogGeneratorConfig;
use crate::models::{ConventionalCommit, SrShared};

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GenerateChangelogConfig {
    commits: Option<Vec<ConventionalCommit>>,
    filter_non_user_facing_commits: bool,
    refine_commits_summary: bool,
    changelog_rules: ChangelogGeneratorConfig,
}

#[derive(Default)]
pub struct GenerateChangelog;

impl Middleware for GenerateChangelog {
    const NAME: &'static str = "generate-changelog";
    const DESCRIPTION: &'static str = "generate structured changelog categories from commits";
    type Config = GenerateChangelogConfig;

    fn execute(&self, ctx: &Context, cfg: GenerateChangelogConfig) -> MiddlewareResult {
        if cfg.filter_non_user_facing_commits || cfg.refine_commits_summary {
            return MiddlewareResult::failure(
                "AI-assisted changelog refinement is not available in this build; set filterNonUserFacingCommits and refineCommitsSummary to false.",
            );
        }

        let commits = cfg
            .commits
            .clone()
            .unwrap_or_else(|| ctx.state::<SrShared>().commits.get());
        if commits.is_empty() {
            ctx.log_warn("No commits provided for changelog generation.");
            return MiddlewareResult::success();
        }

        let categories = cfg.changelog_rules.generate(&commits);
        MiddlewareResult::success_with(move |o| {
            o.set("categories", categories);
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_plugin_sdk::testing::{run, MockHost};
    use moonlit_plugin_sdk::LogLevel;
    use serde_json::Value;

    fn cfg(json: Value) -> GenerateChangelogConfig {
        moonlit_plugin_sdk::config::from_json_value(&json.to_string()).unwrap()
    }
    fn run_it(
        shared: &SrShared,
        host: &MockHost,
        c: GenerateChangelogConfig,
    ) -> moonlit_plugin_sdk::bindings::MiddlewareResult {
        let ctx = Context::new(host, "/w".into(), "s".into()).with_state(shared);
        run(&GenerateChangelog, &ctx, c).into_wit()
    }

    #[test]
    fn ai_flags_fail_fast() {
        let shared = SrShared::default();
        let host = MockHost::new();
        let c = cfg(serde_json::json!({ "filterNonUserFacingCommits": true }));
        let w = run_it(&shared, &host, c);
        assert!(!w.successful);
        assert_eq!(
            w.error_message.as_deref(),
            Some("AI-assisted changelog refinement is not available in this build; set filterNonUserFacingCommits and refineCommitsSummary to false.")
        );
    }

    #[test]
    fn empty_commits_succeed_without_output_and_warn() {
        let shared = SrShared::default();
        let host = MockHost::new();
        let w = run_it(&shared, &host, GenerateChangelogConfig::default());
        assert!(w.successful);
        assert!(w.output.is_empty());
        assert!(host
            .logs()
            .iter()
            .any(|(l, m)| *l == LogLevel::Warn
                && m == "No commits provided for changelog generation."));
    }

    #[test]
    fn emits_categories_from_shared_commits() {
        let shared = SrShared::default();
        shared.commits.set(vec![crate::models::ConventionalCommit {
            kind: "feat".into(),
            summary: "add flag".into(),
            sha: "abc1234".into(),
            ..Default::default()
        }]);
        let host = MockHost::new();
        let w = run_it(&shared, &host, GenerateChangelogConfig::default());
        assert!(w.successful);
        let out: std::collections::HashMap<String, Value> = w
            .output
            .into_iter()
            .map(|(k, v)| (k, serde_json::from_str(&v).unwrap()))
            .collect();
        let cats = out["categories"].as_array().unwrap();
        assert_eq!(cats[0]["name"], "Features");
        assert_eq!(cats[0]["entries"][0]["description"], "add flag");
    }
}
