//! Parse raw commit messages into conventional commits. Regex is applied to the
//! first non-empty, trimmed line of the message (verbatim 1.x behavior).

use regex::Regex;

use crate::models::{Commit, ConventionalCommit};

/// Convert every raw commit. Compiles the regex once per call.
pub fn convert_all(commits: &[Commit]) -> Vec<ConventionalCommit> {
    let re =
        Regex::new(r"^(?P<type>\w+)(?:\((?P<scope>[^)]+)\))?(?P<breaking>!)?:\s*(?P<body>.*)$")
            .expect("valid conventional-commit regex");
    commits.iter().map(|c| convert_one(&re, c)).collect()
}

fn short_sha(sha: &str) -> String {
    sha[..sha.len().min(7)].to_string()
}

fn first_line(message: &str) -> String {
    message
        .split('\n')
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("")
        .to_string()
}

fn contains_ci(haystack: &str, needle: &str) -> bool {
    haystack
        .to_ascii_uppercase()
        .contains(&needle.to_ascii_uppercase())
}

fn convert_one(re: &Regex, c: &Commit) -> ConventionalCommit {
    let first = first_line(&c.message);
    match re.captures(&first) {
        None => ConventionalCommit {
            sha: short_sha(&c.sha),
            summary: first,
            kind: "unknown".to_string(),
            scope: None,
            body: c.message.clone(),
            is_breaking_change: false,
            raw_message: c.message.clone(),
            date: c.date.clone(),
        },
        Some(caps) => {
            let breaking = caps.name("breaking").is_some()
                || contains_ci(&c.message, "BREAKING CHANGE:")
                || contains_ci(&c.message, "BREAKING-CHANGE:");
            ConventionalCommit {
                sha: short_sha(&c.sha),
                summary: caps
                    .name("body")
                    .map(|m| m.as_str().trim().to_string())
                    .unwrap_or_default(),
                kind: caps["type"].to_lowercase(),
                scope: caps.name("scope").map(|m| m.as_str().to_string()),
                body: c.message.clone(),
                is_breaking_change: breaking,
                raw_message: c.message.clone(),
                date: c.date.clone(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Commit;

    fn c(sha: &str, message: &str) -> Commit {
        Commit {
            sha: sha.into(),
            message: message.into(),
            date: "2026-01-01T00:00:00Z".into(),
            ..Default::default()
        }
    }

    #[test]
    fn parses_type_scope_and_summary() {
        let out = convert_all(&[c("abcdef1234567", "feat(cli): add flag")]);
        assert_eq!(out[0].kind, "feat");
        assert_eq!(out[0].scope.as_deref(), Some("cli"));
        assert_eq!(out[0].summary, "add flag");
        assert_eq!(out[0].sha, "abcdef1", "sha truncated to 7 chars");
        assert!(!out[0].is_breaking_change);
    }

    #[test]
    fn lowercases_type_and_trims_summary() {
        let out = convert_all(&[c("aaaaaaa", "FIX:   spacing  ")]);
        assert_eq!(out[0].kind, "fix");
        assert_eq!(out[0].summary, "spacing");
    }

    #[test]
    fn unparseable_falls_back_to_unknown() {
        let out = convert_all(&[c("bbbbbbb", "just a note")]);
        assert_eq!(out[0].kind, "unknown");
        assert_eq!(out[0].summary, "just a note");
        assert_eq!(out[0].scope, None);
        assert!(!out[0].is_breaking_change);
    }

    #[test]
    fn bang_marks_breaking() {
        let out = convert_all(&[c("ccccccc", "feat!: drop v1")]);
        assert!(out[0].is_breaking_change);
        assert_eq!(out[0].kind, "feat");
    }

    #[test]
    fn breaking_change_footer_marks_breaking_case_insensitively() {
        let out = convert_all(&[c("ddddddd", "fix: patch\n\nbreaking change: removed API")]);
        assert!(out[0].is_breaking_change);
        let out2 = convert_all(&[c("eeeeeee", "fix: patch\n\nBREAKING-CHANGE: removed API")]);
        assert!(out2[0].is_breaking_change);
    }

    #[test]
    fn first_non_empty_trimmed_line_is_used() {
        let out = convert_all(&[c("fffffff", "\n\n  feat: after blanks  \nmore")]);
        assert_eq!(out[0].kind, "feat");
        assert_eq!(out[0].summary, "after blanks");
    }

    #[test]
    fn short_sha_guard_does_not_panic_on_tiny_sha() {
        let out = convert_all(&[c("ab", "chore: x")]);
        assert_eq!(out[0].sha, "ab");
    }
}
