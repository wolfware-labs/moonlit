//! Domain models shared across the three middlewares.

use moonlit_plugin_sdk::prelude::*; // Deserialize, Shared
use serde::Serialize;

/// A raw commit as produced by the `git` plugin's `commits.details` output.
/// Only `sha`, `date`, and `message` participate in the algorithm.
#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase", default)]
pub struct Commit {
    pub sha: String,
    pub author: String,
    pub email: String,
    pub date: String,
    pub message: String,
}

impl Default for Commit {
    fn default() -> Self {
        Self {
            sha: String::new(),
            author: String::new(),
            email: String::new(),
            date: String::new(),
            message: String::new(),
        }
    }
}

/// A parsed conventional commit. Emitted by `analyze`, stored in `SrShared`, and
/// consumed by `calculate-version` / `generate-changelog`. `#[serde(default)]` lets
/// config-provided arrays omit fields and round-trips analyze's own output.
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ConventionalCommit {
    pub sha: String,
    pub summary: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub scope: Option<String>,
    pub body: String,
    pub is_breaking_change: bool,
    pub raw_message: String,
    pub date: String,
}

/// Version bump magnitude. Declaration order == `Ord` order (None < Patch < Minor
/// < Major), which the algorithm relies on for "highest bump wins" and
/// "bump <= current-version-level".
#[derive(Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
#[repr(u8)]
pub enum VersionBumpType {
    #[default]
    None = 0,
    Patch = 1,
    Minor = 2,
    Major = 3,
}

/// A version-bump rule: match by type/scope, produce a bump.
#[derive(Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ReleaseRule {
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub scope: Option<String>,
    pub release: VersionBumpType,
}

impl ReleaseRule {
    pub fn new(kind: &str, release: VersionBumpType) -> Self {
        Self { kind: Some(kind.to_string()), scope: None, release }
    }

    pub fn matches(&self, c: &ConventionalCommit) -> bool {
        let type_ok = self
            .kind
            .as_deref()
            .map_or(true, |t| t.is_empty() || t.eq_ignore_ascii_case(&c.kind));
        let scope_ok = self.scope.as_deref().map_or(true, |s| {
            s.is_empty() || c.scope.as_deref().map_or(false, |cs| cs.eq_ignore_ascii_case(s))
        });
        type_ok && scope_ok
    }
}

/// A changelog category rule. `matches` ports 1.x exactly: a breaking commit
/// satisfies the breaking rule's first clause and fails every non-breaking rule's
/// `is_breaking_change == false` clause, so it lands in "Breaking Changes" only.
#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChangelogRule {
    #[serde(rename = "type")]
    pub kind: Option<String>,
    #[serde(default)]
    pub is_breaking_change: bool,
    pub icon: String,
    pub section: String,
    pub summary: String,
}

impl ChangelogRule {
    pub fn matches(&self, c: &ConventionalCommit) -> bool {
        if self.is_breaking_change && c.is_breaking_change {
            return true;
        }
        self.kind
            .as_deref()
            .map_or(false, |t| t.eq_ignore_ascii_case(&c.kind))
            && c.is_breaking_change == self.is_breaking_change
    }
}

/// Plugin-wide shared state — one instance per pipeline run. `analyze` writes the
/// parsed commits; `calculate-version` / `generate-changelog` read them back.
#[derive(Default)]
pub struct SrShared {
    pub commits: Shared<Vec<ConventionalCommit>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit(kind: &str, scope: Option<&str>, breaking: bool) -> ConventionalCommit {
        ConventionalCommit {
            kind: kind.to_string(),
            scope: scope.map(str::to_string),
            is_breaking_change: breaking,
            ..Default::default()
        }
    }

    #[test]
    fn bump_type_orders_none_lt_patch_lt_minor_lt_major() {
        assert!(VersionBumpType::None < VersionBumpType::Patch);
        assert!(VersionBumpType::Patch < VersionBumpType::Minor);
        assert!(VersionBumpType::Minor < VersionBumpType::Major);
    }

    #[test]
    fn release_rule_matches_type_case_insensitively_and_ignores_scope_when_unset() {
        let r = ReleaseRule::new("feat", VersionBumpType::Minor);
        assert!(r.matches(&commit("FEAT", Some("cli"), false)));
        assert!(!r.matches(&commit("fix", None, false)));
    }

    #[test]
    fn release_rule_with_scope_requires_scope_match() {
        let r = ReleaseRule { kind: Some("feat".into()), scope: Some("cli".into()), release: VersionBumpType::Minor };
        assert!(r.matches(&commit("feat", Some("CLI"), false)));
        assert!(!r.matches(&commit("feat", None, false)));
        assert!(!r.matches(&commit("feat", Some("api"), false)));
    }

    #[test]
    fn changelog_rule_breaking_commit_matches_only_breaking_rule() {
        let feat = ChangelogRule { kind: Some("feat".into()), is_breaking_change: false, icon: ":sparkles:".into(), section: "Features".into(), summary: "New features".into() };
        let breaking = ChangelogRule { kind: Some("breaking".into()), is_breaking_change: true, icon: ":boom:".into(), section: "Breaking Changes".into(), summary: "Breaking changes".into() };
        let breaking_feat = commit("feat", None, true);
        assert!(!feat.matches(&breaking_feat), "breaking feat must NOT land in Features");
        assert!(breaking.matches(&breaking_feat), "breaking feat lands in Breaking Changes");
        let plain_feat = commit("feat", None, false);
        assert!(feat.matches(&plain_feat));
        assert!(!breaking.matches(&plain_feat));
    }
}
