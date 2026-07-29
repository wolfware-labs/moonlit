//! `calculate-version` — compute the next semantic version from a base version,
//! branch prerelease mapping, and the commit set. Fully offline.

use std::collections::BTreeMap;

use globset::Glob;
use moonlit_sdk::prelude::*;
use semver::Version;

use crate::models::{ConventionalCommit, SrShared};
use crate::version::{
    calculate_next, with_metadata, with_prerelease, without_metadata, AnalyzerConfig,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CalculateVersionConfig {
    initial_version: String,
    base_version: Option<String>,
    branch: String,
    commits: Option<Vec<ConventionalCommit>>,
    conventional_commit_rules: AnalyzerConfig,
    prerelease_mappings: BTreeMap<String, String>,
}

impl Default for CalculateVersionConfig {
    fn default() -> Self {
        Self {
            initial_version: "1.0.0".to_string(),
            base_version: None,
            branch: String::new(),
            commits: None,
            conventional_commit_rules: AnalyzerConfig::create_default(),
            prerelease_mappings: BTreeMap::new(),
        }
    }
}

#[derive(Default)]
pub struct CalculateVersion;

impl Middleware for CalculateVersion {
    const NAME: &'static str = "calculate-version";
    const DESCRIPTION: &'static str = "calculate the next semantic version";
    type Config = CalculateVersionConfig;

    fn execute(&self, ctx: &Context, cfg: CalculateVersionConfig) -> MiddlewareResult {
        let commits = cfg
            .commits
            .clone()
            .unwrap_or_else(|| ctx.state::<SrShared>().commits.get());
        if commits.is_empty() {
            return MiddlewareResult::failure("No commits provided for version calculation.");
        }

        let metadata = newest_sha_metadata(&commits);
        let suffix = resolve_suffix(&cfg.prerelease_mappings, &cfg.branch);

        let next: Option<Version> = if cfg
            .base_version
            .as_deref()
            .is_none_or(|b| b.trim().is_empty())
        {
            let mut v = match Version::parse(&cfg.initial_version) {
                Ok(v) => v,
                Err(e) => {
                    return MiddlewareResult::failure(format!(
                        "Invalid initialVersion '{}': {e}",
                        cfg.initial_version
                    ))
                }
            };
            if let Some(sfx) = suffix.as_deref() {
                v = with_prerelease(v, sfx, 1);
            }
            Some(with_metadata(v, &metadata))
        } else {
            let raw = cfg.base_version.as_deref().unwrap();
            let base = match Version::parse(raw) {
                Ok(v) => v,
                Err(e) => {
                    return MiddlewareResult::failure(format!("Invalid baseVersion '{raw}': {e}"))
                }
            };
            calculate_next(
                &base,
                &commits,
                suffix.as_deref(),
                &cfg.conventional_commit_rules,
            )
            .map(|v| with_metadata(v, &metadata))
        };

        match next {
            None => MiddlewareResult::success_with(|o| {
                o.set("hasNewVersion", false);
            }),
            Some(v) => {
                let next_version = without_metadata(v.clone()).to_string();
                let next_full = v.to_string();
                let is_pre = !v.pre.is_empty();
                MiddlewareResult::success_with(move |o| {
                    o.set("hasNewVersion", true);
                    o.set("nextVersion", next_version);
                    o.set("nextFullVersion", next_full);
                    o.set("isPrerelease", is_pre);
                })
            }
        }
    }
}

/// `sha-<7 chars of the newest-by-date commit>`. Unparseable dates sort earliest
/// (they never win a tie over a parseable newer commit); ties resolve to the last
/// in input order (`max_by_key` semantics). `commits` is guaranteed non-empty.
fn newest_sha_metadata(commits: &[ConventionalCommit]) -> String {
    let newest = commits
        .iter()
        .max_by_key(|c| chrono::DateTime::parse_from_rfc3339(&c.date).ok())
        .expect("commits is non-empty");
    let sha = &newest.sha[..newest.sha.len().min(7)];
    format!("sha-{sha}")
}

/// Exact key wins (empty value -> stable/None); otherwise the alphabetically-first
/// glob (BTreeMap iterates in sorted order) that matches `branch`.
fn resolve_suffix(map: &BTreeMap<String, String>, branch: &str) -> Option<String> {
    if let Some(v) = map.get(branch) {
        return non_empty(v);
    }
    for (pattern, v) in map {
        if let Ok(glob) = Glob::new(pattern) {
            if glob.compile_matcher().is_match(branch) {
                return non_empty(v);
            }
        }
    }
    None
}

fn non_empty(v: &str) -> Option<String> {
    if v.trim().is_empty() {
        None
    } else {
        Some(v.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_sdk::testing::{run, MockHost};
    use serde_json::Value;

    fn cfg(json: Value) -> CalculateVersionConfig {
        moonlit_sdk::config::from_json_value(&json.to_string()).unwrap()
    }
    fn outputs(
        w: moonlit_sdk::bindings::MiddlewareResult,
    ) -> std::collections::HashMap<String, Value> {
        w.output
            .into_iter()
            .map(|(k, v)| (k, serde_json::from_str(&v).unwrap()))
            .collect()
    }
    fn ctx_run(
        shared: &SrShared,
        c: CalculateVersionConfig,
    ) -> moonlit_sdk::bindings::MiddlewareResult {
        let host = MockHost::new();
        let ctx = Context::new(&host, "/w".into(), "s".into()).with_state(shared);
        run(&CalculateVersion, &ctx, c).into_wit()
    }
    fn commit(sha: &str, kind: &str, date: &str) -> Value {
        serde_json::json!({ "sha": sha, "type": kind, "date": date })
    }

    #[test]
    fn empty_commits_fail_with_exact_message() {
        let shared = SrShared::default();
        let w = ctx_run(&shared, CalculateVersionConfig::default());
        assert!(!w.successful);
        assert_eq!(
            w.error_message.as_deref(),
            Some("No commits provided for version calculation.")
        );
    }

    #[test]
    fn first_release_stable_emits_initial_version() {
        let shared = SrShared::default();
        let c = cfg(
            serde_json::json!({ "commits": [commit("abc1234def", "chore", "2026-01-01T00:00:00Z")] }),
        );
        let w = ctx_run(&shared, c);
        let out = outputs(w);
        assert_eq!(out["hasNewVersion"], serde_json::json!(true));
        assert_eq!(out["nextVersion"], "1.0.0");
        assert_eq!(out["nextFullVersion"], "1.0.0+sha-abc1234");
        assert_eq!(out["isPrerelease"], serde_json::json!(false));
    }

    #[test]
    fn first_release_mapped_label_is_prerelease() {
        let shared = SrShared::default();
        let c = cfg(serde_json::json!({
            "branch": "develop",
            "prereleaseMappings": { "develop": "beta" },
            "commits": [commit("abc1234def", "chore", "2026-01-01T00:00:00Z")]
        }));
        let out = outputs(ctx_run(&shared, c));
        assert_eq!(out["nextVersion"], "1.0.0-beta.1");
        assert_eq!(out["nextFullVersion"], "1.0.0-beta.1+sha-abc1234");
        assert_eq!(out["isPrerelease"], serde_json::json!(true));
    }

    #[test]
    fn no_bump_emits_has_new_version_false_only() {
        let shared = SrShared::default();
        let c = cfg(serde_json::json!({
            "baseVersion": "1.2.3",
            "commits": [commit("abc1234def", "chore", "2026-01-01T00:00:00Z")]
        }));
        let out = outputs(ctx_run(&shared, c));
        assert_eq!(out["hasNewVersion"], serde_json::json!(false));
        assert!(!out.contains_key("nextVersion"));
    }

    #[test]
    fn feat_bump_from_base() {
        let shared = SrShared::default();
        let c = cfg(serde_json::json!({
            "baseVersion": "1.2.3",
            "commits": [commit("abc1234def", "feat", "2026-01-01T00:00:00Z")]
        }));
        let out = outputs(ctx_run(&shared, c));
        assert_eq!(out["nextVersion"], "1.3.0");
        assert_eq!(out["nextFullVersion"], "1.3.0+sha-abc1234");
    }

    #[test]
    fn commits_fall_back_to_shared_state() {
        let shared = SrShared::default();
        shared.commits.set(vec![crate::models::ConventionalCommit {
            sha: "deadbee".into(),
            kind: "feat".into(),
            date: "2026-01-01T00:00:00Z".into(),
            ..Default::default()
        }]);
        let c = cfg(serde_json::json!({ "baseVersion": "1.2.3" }));
        let out = outputs(ctx_run(&shared, c));
        assert_eq!(out["nextVersion"], "1.3.0");
        assert_eq!(out["nextFullVersion"], "1.3.0+sha-deadbee");
    }

    #[test]
    fn metadata_uses_newest_commit_by_date() {
        let shared = SrShared::default();
        let c = cfg(serde_json::json!({
            "baseVersion": "1.2.3",
            "commits": [
                commit("oldold0", "feat", "2026-01-01T00:00:00Z"),
                commit("new1234", "feat", "2026-03-01T00:00:00Z")
            ]
        }));
        let out = outputs(ctx_run(&shared, c));
        assert_eq!(out["nextFullVersion"], "1.3.0+sha-new1234");
    }

    #[test]
    fn glob_mapping_matches_branch() {
        let shared = SrShared::default();
        let c = cfg(serde_json::json!({
            "baseVersion": "1.2.3", "branch": "feature/x",
            "prereleaseMappings": { "feature/*": "beta" },
            "commits": [commit("abc1234def", "feat", "2026-01-01T00:00:00Z")]
        }));
        let out = outputs(ctx_run(&shared, c));
        assert_eq!(out["nextVersion"], "1.3.0-beta.1");
    }

    #[test]
    fn exact_mapping_wins_over_glob_and_empty_is_stable() {
        let shared = SrShared::default();
        let c = cfg(serde_json::json!({
            "baseVersion": "1.2.3", "branch": "feature/x",
            "prereleaseMappings": { "feature/x": "", "feature/*": "beta" },
            "commits": [commit("abc1234def", "feat", "2026-01-01T00:00:00Z")]
        }));
        let out = outputs(ctx_run(&shared, c));
        assert_eq!(out["nextVersion"], "1.3.0");
        assert_eq!(out["isPrerelease"], serde_json::json!(false));
    }

    #[test]
    fn alphabetically_first_glob_wins_when_both_match() {
        let shared = SrShared::default();
        // "r*" and "release/*" both match "release/1"; "r*" sorts first (BTreeMap order).
        let c = cfg(serde_json::json!({
            "baseVersion": "1.2.3", "branch": "release/1",
            "prereleaseMappings": { "r*": "alpha", "release/*": "beta" },
            "commits": [commit("abc1234def", "feat", "2026-01-01T00:00:00Z")]
        }));
        let out = outputs(ctx_run(&shared, c));
        assert_eq!(out["nextVersion"], "1.3.0-alpha.1");
    }
}
