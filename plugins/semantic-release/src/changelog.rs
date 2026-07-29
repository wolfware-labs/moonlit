//! Structured changelog generation. Emits `sdk::changelog::Category` values so the
//! producer shape is byte-identical to what github/gitlab `create-release` consume.

use moonlit_sdk::changelog::{Category, Entry};
use moonlit_sdk::prelude::Deserialize;

use crate::models::{ChangelogRule, ConventionalCommit};

/// Category rules. Defaults to `create_default` (the exact 1.x rule set).
#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChangelogGeneratorConfig {
    #[serde(default = "default_rules")]
    pub rules: Vec<ChangelogRule>,
}

impl Default for ChangelogGeneratorConfig {
    fn default() -> Self {
        Self::create_default()
    }
}

impl ChangelogGeneratorConfig {
    pub fn create_default() -> Self {
        Self {
            rules: default_rules(),
        }
    }

    pub fn generate(&self, commits: &[ConventionalCommit]) -> Vec<Category> {
        let mut categories = Vec::new();
        for rule in &self.rules {
            let entries: Vec<Entry> = commits
                .iter()
                .filter(|c| rule.matches(c))
                .map(entry_from)
                .collect();
            if !entries.is_empty() {
                categories.push(Category {
                    name: rule.section.clone(),
                    icon: rule.icon.clone(),
                    summary: rule.summary.clone(),
                    entries,
                });
            }
        }
        categories
    }
}

fn entry_from(c: &ConventionalCommit) -> Entry {
    let description = match c.scope.as_deref() {
        Some(s) if !s.trim().is_empty() => format!("**{s}**: {}", c.summary),
        _ => c.summary.clone(),
    };
    Entry {
        sha: c.sha.clone(),
        description,
    }
}

fn rule(kind: &str, section: &str, icon: &str, summary: &str) -> ChangelogRule {
    ChangelogRule {
        kind: Some(kind.to_string()),
        is_breaking_change: false,
        icon: icon.to_string(),
        section: section.to_string(),
        summary: summary.to_string(),
    }
}

/// The exact 1.x default rule set, in output order.
fn default_rules() -> Vec<ChangelogRule> {
    vec![
        rule("feat", "Features", ":sparkles:", "New features"),
        rule("fix", "Bug Fixes", ":bug:", "Bug fixes"),
        rule(
            "perf",
            "Performance Improvements",
            ":zap:",
            "Performance improvements",
        ),
        rule("refactor", "Code Refactoring", ":art:", "Code refactoring"),
        rule(
            "style",
            "Code Style Changes",
            ":lipstick:",
            "Code style changes (formatting, missing semi-colons, etc.)",
        ),
        rule(
            "test",
            "Tests",
            ":white_check_mark:",
            "Adding missing tests or correcting existing tests",
        ),
        rule(
            "chore",
            "Chores",
            ":wrench:",
            "Other changes that don't modify src or test files",
        ),
        rule(
            "docs",
            "Documentation",
            ":book:",
            "Documentation only changes",
        ),
        rule(
            "build",
            "Build System",
            ":construction_worker:",
            "Changes that affect the build system or external dependencies",
        ),
        rule(
            "ci",
            "Continuous Integration",
            ":green_heart:",
            "Changes to our CI configuration files and scripts",
        ),
        rule("revert", "Reverts", ":rewind:", "Reverts a previous commit"),
        ChangelogRule {
            kind: Some("breaking".to_string()),
            is_breaking_change: true,
            icon: ":boom:".to_string(),
            section: "Breaking Changes".to_string(),
            summary: "Breaking changes".to_string(),
        },
        rule(
            "unknown",
            "Other Changes",
            ":package:",
            "Other changes that don't fit into the above categories",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ConventionalCommit;

    fn commit(
        kind: &str,
        scope: Option<&str>,
        breaking: bool,
        summary: &str,
        sha: &str,
    ) -> ConventionalCommit {
        ConventionalCommit {
            kind: kind.into(),
            scope: scope.map(str::to_string),
            is_breaking_change: breaking,
            summary: summary.into(),
            sha: sha.into(),
            ..Default::default()
        }
    }

    #[test]
    fn breaking_feat_lands_only_in_breaking_changes() {
        let cats = ChangelogGeneratorConfig::create_default()
            .generate(&[commit("feat", None, true, "drop v1", "aaaaaaa")]);
        let names: Vec<&str> = cats.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"Breaking Changes"));
        assert!(
            !names.contains(&"Features"),
            "breaking feat must not appear under Features"
        );
    }

    #[test]
    fn scoped_entry_description_is_bolded() {
        let cats = ChangelogGeneratorConfig::create_default().generate(&[commit(
            "feat",
            Some("cli"),
            false,
            "add flag",
            "bbbbbbb",
        )]);
        let feat = cats.iter().find(|c| c.name == "Features").unwrap();
        assert_eq!(feat.entries[0].description, "**cli**: add flag");
        assert_eq!(feat.entries[0].sha, "bbbbbbb");
    }

    #[test]
    fn unscoped_entry_description_is_plain() {
        let cats = ChangelogGeneratorConfig::create_default()
            .generate(&[commit("fix", None, false, "patch it", "ccccccc")]);
        let fixes = cats.iter().find(|c| c.name == "Bug Fixes").unwrap();
        assert_eq!(fixes.entries[0].description, "patch it");
        assert_eq!(fixes.icon, ":bug:");
        assert_eq!(fixes.summary, "Bug fixes");
    }

    #[test]
    fn categories_follow_rule_order_and_skip_empties() {
        let cats = ChangelogGeneratorConfig::create_default().generate(&[
            commit("fix", None, false, "b", "1111111"),
            commit("feat", None, false, "a", "2222222"),
        ]);
        let names: Vec<&str> = cats.iter().map(|c| c.name.as_str()).collect();
        // feat rule precedes fix rule -> Features before Bug Fixes; no empty categories
        assert_eq!(names, vec!["Features", "Bug Fixes"]);
    }

    #[test]
    fn no_matching_commits_yields_empty_vec() {
        let cats = ChangelogGeneratorConfig::create_default().generate(&[]);
        assert!(cats.is_empty());
    }

    #[test]
    fn omitted_rules_preserve_default_rule_set() {
        let cfg: ChangelogGeneratorConfig =
            moonlit_sdk::config::from_json_value(&serde_json::json!({}).to_string())
                .unwrap();
        let cats = cfg.generate(&[commit("feat", None, false, "add flag", "abc1234")]);
        let names: Vec<&str> = cats.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"Features"));
    }

    #[test]
    fn explicit_empty_rules_yield_no_categories() {
        let cfg: ChangelogGeneratorConfig = moonlit_sdk::config::from_json_value(
            &serde_json::json!({ "rules": [] }).to_string(),
        )
        .unwrap();
        let cats = cfg.generate(&[commit("feat", None, false, "add flag", "abc1234")]);
        assert!(cats.is_empty());
    }
}
