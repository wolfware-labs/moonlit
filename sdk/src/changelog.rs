//! Shared release-notes markdown generator. github (§11.2) is the first
//! consumer; gitlab (§11.3) reuses it with a different `commit_url_prefix`.

use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct Entry {
    pub sha: String,
    pub description: String,
}

#[derive(Deserialize, Clone)]
pub struct Category {
    pub name: String,
    pub icon: String,
    pub summary: String,
    #[serde(default)]
    pub entries: Vec<Entry>,
}

/// Render categories to markdown. Empty categories are skipped. A commit links
/// to `{commit_url_prefix}{sha}`; the visible short SHA is the first 7 chars.
/// Categories are separated by one blank line; output ends with a single `\n`.
pub fn render(categories: &[Category], commit_url_prefix: &str) -> String {
    let mut blocks: Vec<String> = Vec::new();
    for cat in categories {
        if cat.entries.is_empty() {
            continue;
        }
        let mut block = String::new();
        block.push_str(&format!("## {} {}\n", cat.icon, cat.name));
        block.push_str(&format!("#### {}\n", cat.summary));
        for e in &cat.entries {
            let sha7 = &e.sha[..e.sha.len().min(7)];
            block.push_str(&format!(
                "- {} ([{}]({}{}))\n",
                e.description, sha7, commit_url_prefix, e.sha
            ));
        }
        blocks.push(block);
    }
    blocks.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cat(name: &str, icon: &str, summary: &str, entries: &[(&str, &str)]) -> Category {
        Category {
            name: name.into(),
            icon: icon.into(),
            summary: summary.into(),
            entries: entries
                .iter()
                .map(|(sha, desc)| Entry {
                    sha: (*sha).into(),
                    description: (*desc).into(),
                })
                .collect(),
        }
    }

    #[test]
    fn renders_two_categories_with_blank_line_between() {
        let cats = vec![
            cat(
                "Features",
                ":sparkles:",
                "New features",
                &[("abc1234def", "**cli**: add flag")],
            ),
            cat(
                "Bug Fixes",
                ":bug:",
                "Fixes",
                &[("def5678abc", "fix crash")],
            ),
        ];
        let md = render(&cats, "https://github.com/o/r/commit/");
        assert_eq!(
            md,
            "## :sparkles: Features\n\
             #### New features\n\
             - **cli**: add flag ([abc1234](https://github.com/o/r/commit/abc1234def))\n\
             \n\
             ## :bug: Bug Fixes\n\
             #### Fixes\n\
             - fix crash ([def5678](https://github.com/o/r/commit/def5678abc))\n"
        );
    }

    #[test]
    fn single_category_has_no_leading_or_trailing_blank() {
        let md = render(
            &[cat("Features", ":sparkles:", "New", &[("0123456789", "x")])],
            "https://github.com/o/r/commit/",
        );
        assert_eq!(
            md,
            "## :sparkles: Features\n#### New\n- x ([0123456](https://github.com/o/r/commit/0123456789))\n"
        );
    }

    #[test]
    fn empty_categories_are_skipped() {
        let cats = vec![
            cat("Empty", ":ghost:", "none", &[]),
            cat("Features", ":sparkles:", "New", &[("abcdef1234", "y")]),
        ];
        let md = render(&cats, "https://github.com/o/r/commit/");
        assert!(!md.contains("Empty"));
        assert!(md.starts_with("## :sparkles: Features\n"));
    }
}
