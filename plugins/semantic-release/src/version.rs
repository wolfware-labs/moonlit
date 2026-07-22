//! The pure version engine: bump analysis + semver arithmetic helpers +
//! `calculate_next`. No I/O — the golden tests exercise every branch.

use semver::{BuildMetadata, Prerelease, Version};
use moonlit_plugin_sdk::prelude::Deserialize;

use crate::models::{ConventionalCommit, ReleaseRule, VersionBumpType};

fn default_true() -> bool {
    true
}

/// Bump rules + the "breaking is always major" switch. Defaults to `create_default`.
#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzerConfig {
    #[serde(default = "default_true")]
    pub breaking_changes_always_major: bool,
    #[serde(default)]
    pub rules: Vec<ReleaseRule>,
}

impl Default for AnalyzerConfig {
    fn default() -> Self {
        Self::create_default()
    }
}

impl AnalyzerConfig {
    pub fn create_default() -> Self {
        Self {
            breaking_changes_always_major: true,
            rules: vec![
                ReleaseRule::new("feat", VersionBumpType::Minor),
                ReleaseRule::new("fix", VersionBumpType::Patch),
                ReleaseRule::new("perf", VersionBumpType::Patch),
                ReleaseRule::new("revert", VersionBumpType::Patch),
                ReleaseRule::new("docs", VersionBumpType::None),
                ReleaseRule::new("style", VersionBumpType::None),
                ReleaseRule::new("chore", VersionBumpType::None),
                ReleaseRule::new("refactor", VersionBumpType::None),
                ReleaseRule::new("test", VersionBumpType::None),
                ReleaseRule::new("build", VersionBumpType::None),
                ReleaseRule::new("ci", VersionBumpType::None),
            ],
        }
    }

    /// Highest per-commit bump across all commits.
    pub fn analyze(&self, commits: &[ConventionalCommit]) -> VersionBumpType {
        commits
            .iter()
            .map(|c| self.determine_bump(c))
            .max()
            .unwrap_or(VersionBumpType::None)
    }

    fn determine_bump(&self, c: &ConventionalCommit) -> VersionBumpType {
        if self.breaking_changes_always_major && c.is_breaking_change {
            return VersionBumpType::Major;
        }
        self.rules
            .iter()
            .find(|r| r.matches(c))
            .map(|r| r.release)
            .unwrap_or(VersionBumpType::None)
    }
}

pub fn bumped(v: &Version, bump: VersionBumpType) -> Version {
    match bump {
        VersionBumpType::Major => Version::new(v.major + 1, 0, 0),
        VersionBumpType::Minor => Version::new(v.major, v.minor + 1, 0),
        VersionBumpType::Patch => Version::new(v.major, v.minor, v.patch + 1),
        VersionBumpType::None => unreachable!("bumped() is never called with None"),
    }
}

pub fn version_level(v: &Version) -> VersionBumpType {
    if v.major > 0 && v.minor == 0 && v.patch == 0 {
        VersionBumpType::Major
    } else if v.minor > 0 && v.patch == 0 {
        VersionBumpType::Minor
    } else {
        VersionBumpType::Patch
    }
}

pub fn prerelease_info(v: &Version) -> (String, i64) {
    if v.pre.is_empty() {
        return (String::new(), 0);
    }
    let parts: Vec<&str> = v.pre.as_str().split('.').collect();
    if parts.len() < 2 {
        return (String::new(), 0);
    }
    (parts[0].to_string(), parts[1].parse::<i64>().unwrap_or(0))
}

pub fn with_prerelease(mut v: Version, label: &str, iteration: i64) -> Version {
    v.pre = Prerelease::new(&format!("{label}.{iteration}")).expect("valid prerelease identifier");
    v
}

pub fn without_prerelease(mut v: Version) -> Version {
    v.pre = Prerelease::EMPTY;
    v
}

pub fn with_metadata(mut v: Version, meta: &str) -> Version {
    v.build = BuildMetadata::new(meta).expect("valid build metadata");
    v
}

pub fn without_metadata(mut v: Version) -> Version {
    v.build = BuildMetadata::EMPTY;
    v
}

/// Port of 1.x `SemanticVersionCalculator.CalculateNextVersion`. `suffix == None`
/// is the stable channel; `Some(label)` is a prerelease channel. Returns `None`
/// when the commit set implies no bump. Callers guarantee `commits` is non-empty.
pub fn calculate_next(
    base: &Version,
    commits: &[ConventionalCommit],
    suffix: Option<&str>,
    analyzer: &AnalyzerConfig,
) -> Option<Version> {
    let bump = analyzer.analyze(commits);
    if bump == VersionBumpType::None {
        return None;
    }

    match suffix {
        None => Some(if base.pre.is_empty() {
            bumped(base, bump)
        } else {
            without_prerelease(base.clone())
        }),
        Some(sfx) => {
            let (label, iteration) = prerelease_info(base);
            let level = version_level(base);
            if label == sfx && bump <= level {
                let v = Version::new(base.major, base.minor, base.patch);
                Some(with_prerelease(v, &label, iteration + 1))
            } else {
                Some(with_prerelease(bumped(base, bump), sfx, 1))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ConventionalCommit;
    use semver::Version;

    fn c(kind: &str, breaking: bool) -> ConventionalCommit {
        ConventionalCommit { kind: kind.into(), is_breaking_change: breaking, ..Default::default() }
    }

    #[test]
    fn default_rules_map_types_to_bumps() {
        let a = AnalyzerConfig::create_default();
        assert_eq!(a.analyze(&[c("feat", false)]), VersionBumpType::Minor);
        assert_eq!(a.analyze(&[c("fix", false)]), VersionBumpType::Patch);
        assert_eq!(a.analyze(&[c("perf", false)]), VersionBumpType::Patch);
        assert_eq!(a.analyze(&[c("revert", false)]), VersionBumpType::Patch);
        assert_eq!(a.analyze(&[c("chore", false)]), VersionBumpType::None);
        assert_eq!(a.analyze(&[c("docs", false)]), VersionBumpType::None);
        assert_eq!(a.analyze(&[c("unknown", false)]), VersionBumpType::None);
    }

    #[test]
    fn breaking_forces_major_and_highest_bump_wins() {
        let a = AnalyzerConfig::create_default();
        assert_eq!(a.analyze(&[c("fix", true)]), VersionBumpType::Major);
        assert_eq!(a.analyze(&[c("chore", false), c("feat", false), c("fix", false)]), VersionBumpType::Minor);
    }

    #[test]
    fn version_level_classifies_base() {
        assert_eq!(version_level(&Version::parse("2.0.0").unwrap()), VersionBumpType::Major);
        assert_eq!(version_level(&Version::parse("1.2.0").unwrap()), VersionBumpType::Minor);
        assert_eq!(version_level(&Version::parse("1.2.3").unwrap()), VersionBumpType::Patch);
    }

    #[test]
    fn prerelease_info_extracts_label_and_iteration() {
        assert_eq!(prerelease_info(&Version::parse("1.0.0-beta.3").unwrap()), ("beta".to_string(), 3));
        assert_eq!(prerelease_info(&Version::parse("1.0.0").unwrap()), (String::new(), 0));
        assert_eq!(prerelease_info(&Version::parse("1.0.0-rc").unwrap()), (String::new(), 0));
    }

    #[test]
    fn helpers_build_versions() {
        let v = Version::parse("1.2.3").unwrap();
        assert_eq!(bumped(&v, VersionBumpType::Major).to_string(), "2.0.0");
        assert_eq!(bumped(&v, VersionBumpType::Minor).to_string(), "1.3.0");
        assert_eq!(bumped(&v, VersionBumpType::Patch).to_string(), "1.2.4");
        assert_eq!(with_prerelease(v.clone(), "beta", 1).to_string(), "1.2.3-beta.1");
        assert_eq!(with_metadata(v.clone(), "sha-abc1234").to_string(), "1.2.3+sha-abc1234");
        let pre = Version::parse("1.2.3-beta.1+sha-x").unwrap();
        assert_eq!(without_prerelease(pre.clone()).to_string(), "1.2.3+sha-x");
        assert_eq!(without_metadata(pre).to_string(), "1.2.3-beta.1");
    }

    fn feats(n: usize) -> Vec<ConventionalCommit> {
        (0..n).map(|_| c("feat", false)).collect()
    }

    fn one(kind: &str, breaking: bool) -> Vec<ConventionalCommit> {
        vec![c(kind, breaking)]
    }

    fn calc(base: &str, suffix: Option<&str>, commits: &[ConventionalCommit]) -> Option<String> {
        let a = AnalyzerConfig::create_default();
        calculate_next(&Version::parse(base).unwrap(), commits, suffix, &a).map(|v| v.to_string())
    }

    #[test]
    fn no_bump_returns_none() {
        assert_eq!(calc("1.2.3", None, &one("chore", false)), None);
    }

    #[test]
    fn stable_channel_numeric_bumps() {
        assert_eq!(calc("1.2.3", None, &one("feat", false)).as_deref(), Some("1.3.0"));
        assert_eq!(calc("1.2.3", None, &one("fix", false)).as_deref(), Some("1.2.4"));
        assert_eq!(calc("1.2.3", None, &one("feat", true)).as_deref(), Some("2.0.0"));
    }

    #[test]
    fn stable_channel_promotes_prerelease_without_numeric_bump() {
        // base is a prerelease, target is stable -> strip prerelease, no numeric bump
        assert_eq!(calc("1.3.0-beta.2", None, &one("feat", false)).as_deref(), Some("1.3.0"));
    }

    #[test]
    fn prerelease_iterates_when_bump_within_level() {
        // base 2.0.0-beta.1 (level Major), feat (Minor) <= Major -> iterate
        assert_eq!(calc("2.0.0-beta.1", Some("beta"), &one("feat", false)).as_deref(), Some("2.0.0-beta.2"));
    }

    #[test]
    fn prerelease_numeric_bumps_and_restarts_when_bump_exceeds_level() {
        // base 1.2.0-beta.3 (level Minor), feat! (Major) > Minor -> bump to 2.0.0, restart .1
        assert_eq!(calc("1.2.0-beta.3", Some("beta"), &one("feat", true)).as_deref(), Some("2.0.0-beta.1"));
    }

    #[test]
    fn channel_switch_bumps_and_restarts_at_one() {
        // base 1.2.0-alpha.2, target label beta (differs) -> numeric bump + beta.1
        assert_eq!(calc("1.2.0-alpha.2", Some("beta"), &one("feat", false)).as_deref(), Some("1.3.0-beta.1"));
    }

    #[test]
    fn multiple_commits_use_highest_bump() {
        assert_eq!(calc("1.2.3", None, &feats(3)).as_deref(), Some("1.3.0"));
    }
}
