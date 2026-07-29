//! Provider-agnostic AI refinement of the commit list, before changelog generation.
//! Batches commits (15/batch, sequential), talks only to the `ChatClient` trait.

use moonlit_plugin_sdk::prelude::*;

use crate::ai::{ChatClient, ChatError, ChatRequest};
use crate::models::ConventionalCommit;

pub const BATCH: usize = 15;

pub const FILTER_SYSTEM: &str = "You are a release-notes editor. You receive a JSON array of \
commits, each with an index, type, scope, and summary. Identify the commits that are NOT \
user-facing — internal chores, CI/build tweaks, and refactors or test changes with no \
user-visible effect. Respond with JSON only, no prose, in the exact shape \
{\"drop\": [<index>, ...]} listing the indices to drop. If every commit is user-facing, \
return {\"drop\": []}.";

/// One batch's user-message payload: a JSON array of `{index,type,scope,summary}`.
pub fn batch_items_json(batch: &[ConventionalCommit]) -> String {
    let items: Vec<serde_json::Value> = batch
        .iter()
        .enumerate()
        .map(|(i, c)| {
            serde_json::json!({
                "index": i,
                "type": c.kind,
                "scope": c.scope,
                "summary": c.summary,
            })
        })
        .collect();
    serde_json::Value::Array(items).to_string()
}

/// Turn a ChatError into a descriptive step-failure message.
pub fn err_to_string(e: ChatError) -> String {
    match e {
        ChatError::Auth(m) => m,
        ChatError::RateLimited { .. } => "OpenAI request failed: rate limited after retries.".into(),
        ChatError::Transport(m) => format!("OpenAI request failed: {m}."),
        ChatError::Malformed(m) => format!("OpenAI returned an unparseable response: {m}."),
    }
}

#[derive(serde::Deserialize)]
struct DropReply {
    #[serde(default)]
    drop: Vec<usize>,
}

/// Drop non-user-facing commits, batch by batch. Errors (→ step failure) on any
/// provider error, unparseable reply, or out-of-range index.
pub fn filter_commits(
    ctx: &Context,
    client: &dyn ChatClient,
    commits: Vec<ConventionalCommit>,
) -> Result<Vec<ConventionalCommit>, String> {
    let mut kept = Vec::new();
    for batch in commits.chunks(BATCH) {
        let req = ChatRequest {
            system: FILTER_SYSTEM.to_string(),
            user: batch_items_json(batch),
        };
        let resp = client.complete(ctx, &req).map_err(err_to_string)?;
        let reply: DropReply = serde_json::from_str(&resp.text)
            .map_err(|e| format!("OpenAI returned an unparseable response: {e}."))?;
        for &idx in &reply.drop {
            if idx >= batch.len() {
                return Err(format!("OpenAI returned an out-of-range index {idx}."));
            }
        }
        for (i, c) in batch.iter().enumerate() {
            if !reply.drop.contains(&i) {
                kept.push(c.clone());
            }
        }
    }
    Ok(kept)
}

pub const REFINE_SYSTEM: &str = "You are a release-notes editor. You receive a JSON array of \
commits, each with an index, type, scope, and summary. Rewrite each summary into a concise, \
clear, user-facing release-note line (imperative mood, no trailing period, no type prefix). \
Respond with JSON only, no prose, in the exact shape \
{\"summaries\": [{\"index\": <index>, \"summary\": \"<rewritten>\"}, ...]} covering every input index.";

#[derive(serde::Deserialize)]
struct SummaryReply {
    #[serde(default)]
    summaries: Vec<SummaryItem>,
}
#[derive(serde::Deserialize)]
struct SummaryItem {
    index: usize,
    summary: String,
}

/// Rewrite commit summaries, batch by batch. Commits the model omits are left
/// unchanged. Errors (→ step failure) on provider error, unparseable reply, or
/// out-of-range index.
pub fn refine_summaries(
    ctx: &Context,
    client: &dyn ChatClient,
    commits: Vec<ConventionalCommit>,
) -> Result<Vec<ConventionalCommit>, String> {
    let mut out = commits;
    for start in (0..out.len()).step_by(BATCH) {
        let end = (start + BATCH).min(out.len());
        let batch_len = end - start;
        // Build the payload from an immutable borrow that ends before we mutate `out`.
        let user = batch_items_json(&out[start..end]);
        let req = ChatRequest {
            system: REFINE_SYSTEM.to_string(),
            user,
        };
        let resp = client.complete(ctx, &req).map_err(err_to_string)?;
        let reply: SummaryReply = serde_json::from_str(&resp.text)
            .map_err(|e| format!("OpenAI returned an unparseable response: {e}."))?;
        for item in reply.summaries {
            if item.index >= batch_len {
                return Err(format!("OpenAI returned an out-of-range index {}.", item.index));
            }
            out[start + item.index].summary = item.summary;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::{ChatError, ChatRequest, ChatResponse};
    use crate::models::ConventionalCommit;
    use moonlit_plugin_sdk::testing::MockHost;
    use std::cell::RefCell;

    struct Canned {
        replies: RefCell<Vec<String>>,
        seen_users: RefCell<Vec<String>>,
    }
    impl Canned {
        fn new(replies: Vec<&str>) -> Self {
            Self {
                replies: RefCell::new(replies.into_iter().map(String::from).collect()),
                seen_users: RefCell::new(vec![]),
            }
        }
    }
    impl crate::ai::ChatClient for Canned {
        fn complete(&self, _ctx: &Context, req: &ChatRequest) -> Result<ChatResponse, ChatError> {
            self.seen_users.borrow_mut().push(req.user.clone());
            let mut r = self.replies.borrow_mut();
            Ok(ChatResponse { text: r.remove(0) })
        }
    }
    fn commit(kind: &str, summary: &str) -> ConventionalCommit {
        ConventionalCommit { kind: kind.into(), summary: summary.into(), sha: "abc1234".into(), ..Default::default() }
    }
    fn ctx<'a>(h: &'a MockHost) -> Context<'a> { Context::new(h, "/w".into(), "s".into()) }

    #[test]
    fn filter_drops_flagged_indices() {
        let host = MockHost::new();
        let client = Canned::new(vec![r#"{"drop":[1]}"#]);
        let commits = vec![commit("feat", "add x"), commit("chore", "bump deps"), commit("fix", "y")];
        let out = filter_commits(&ctx(&host), &client, commits).unwrap();
        let kinds: Vec<_> = out.iter().map(|c| c.kind.as_str()).collect();
        assert_eq!(kinds, vec!["feat", "fix"]);
    }

    #[test]
    fn filter_batches_in_groups_of_15() {
        let host = MockHost::new();
        let client = Canned::new(vec![r#"{"drop":[]}"#, r#"{"drop":[]}"#]);
        let commits: Vec<_> = (0..16).map(|i| commit("feat", &format!("c{i}"))).collect();
        let out = filter_commits(&ctx(&host), &client, commits).unwrap();
        assert_eq!(out.len(), 16);
        assert_eq!(client.seen_users.borrow().len(), 2); // 15 + 1
    }

    #[test]
    fn filter_bad_index_errors() {
        let host = MockHost::new();
        let client = Canned::new(vec![r#"{"drop":[9]}"#]); // out of range for 1 commit
        let commits = vec![commit("feat", "only")];
        assert!(filter_commits(&ctx(&host), &client, commits).is_err());
    }

    #[test]
    fn filter_non_json_errors() {
        let host = MockHost::new();
        let client = Canned::new(vec!["not json at all"]);
        let commits = vec![commit("feat", "x")];
        assert!(filter_commits(&ctx(&host), &client, commits).is_err());
    }

    #[test]
    fn refine_rewrites_by_index() {
        let host = MockHost::new();
        let client = Canned::new(vec![r#"{"summaries":[{"index":0,"summary":"Add retry support"}]}"#]);
        let commits = vec![commit("feat", "add retry")];
        let out = refine_summaries(&ctx(&host), &client, commits).unwrap();
        assert_eq!(out[0].summary, "Add retry support");
    }

    #[test]
    fn refine_leaves_unmentioned_commits_unchanged() {
        let host = MockHost::new();
        let client = Canned::new(vec![r#"{"summaries":[{"index":0,"summary":"Rewritten"}]}"#]);
        let commits = vec![commit("feat", "one"), commit("fix", "two")];
        let out = refine_summaries(&ctx(&host), &client, commits).unwrap();
        assert_eq!(out[0].summary, "Rewritten");
        assert_eq!(out[1].summary, "two");
    }

    #[test]
    fn refine_batches_in_groups_of_15() {
        let host = MockHost::new();
        let client = Canned::new(vec![r#"{"summaries":[]}"#, r#"{"summaries":[]}"#]);
        let commits: Vec<_> = (0..16).map(|i| commit("feat", &format!("c{i}"))).collect();
        let out = refine_summaries(&ctx(&host), &client, commits).unwrap();
        assert_eq!(out.len(), 16);
        assert_eq!(client.seen_users.borrow().len(), 2);
    }

    #[test]
    fn refine_bad_index_errors() {
        let host = MockHost::new();
        let client = Canned::new(vec![r#"{"summaries":[{"index":9,"summary":"x"}]}"#]);
        let commits = vec![commit("feat", "only")];
        assert!(refine_summaries(&ctx(&host), &client, commits).is_err());
    }
}
