//! M2 acceptance gate: the committed git.wasm, driven through the real engine
//! host against a throwaway git repo + bare remote. Requires `git` on PATH.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use moonlit_engine::config::model::{FilesystemAccess, Permissions};
use moonlit_engine::host::{
    HostEventSink, InstanceConfig, LogLevel, PluginInstance, ReleaseContext,
};

const FIXTURE: &[u8] = include_bytes!("fixtures/git.wasm");

struct NullSink;
impl HostEventSink for NullSink {
    fn log(&self, _step: &str, _level: LogLevel, _message: &str) {}
    fn progress(&self, _step: &str, _message: &str) {}
}

fn git(dir: &Path, args: &[&str]) {
    let status = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git runs");
    assert!(status.success(), "git {args:?} failed");
}

/// A repo with: one commit, annotated tag v1.0.0, a second commit, and a bare
/// `origin` remote to push to. Returns (repo tempdir, bare tempdir).
fn setup_repo() -> (tempfile::TempDir, tempfile::TempDir) {
    let repo = tempfile::tempdir().unwrap();
    let bare = tempfile::tempdir().unwrap();
    git(bare.path(), &["init", "--bare"]);
    let dir = repo.path();
    git(dir, &["init", "-b", "main"]);
    git(dir, &["config", "user.email", "t@example.com"]);
    git(dir, &["config", "user.name", "Test"]);
    git(
        dir,
        &["remote", "add", "origin", bare.path().to_str().unwrap()],
    );
    fs::write(dir.join("a.txt"), "1").unwrap();
    git(dir, &["add", "."]);
    git(dir, &["commit", "-m", "feat: first"]);
    git(dir, &["tag", "-a", "v1.0.0", "-m", "release 1.0.0"]);
    fs::write(dir.join("b.txt"), "2").unwrap();
    git(dir, &["add", "."]);
    git(dir, &["commit", "-m", "fix: second"]);
    (repo, bare)
}

fn cfg(repo: &Path) -> InstanceConfig {
    InstanceConfig {
        working_directory: repo.to_path_buf(),
        permissions: Permissions {
            network: vec![],
            exec: vec!["git".to_string()],
            env: vec!["*".to_string()],
            filesystem: FilesystemAccess::ReadWrite,
        },
        config_view: serde_json::json!({}),
        env_snapshot: vec![],
    }
}

fn ctx(repo: &Path, step: &str) -> ReleaseContext {
    ReleaseContext {
        working_directory: repo.to_str().unwrap().to_string(),
        step_name: step.to_string(),
    }
}

async fn instance(repo: &Path) -> PluginInstance {
    let eng = moonlit_engine::host::test_engine();
    let mut p = PluginInstance::instantiate(&eng, FIXTURE, cfg(repo), Arc::new(NullSink))
        .await
        .expect("git plugin instantiates");
    p.init(&serde_json::json!({})).await.expect("init Ok");
    p
}

#[tokio::test]
async fn init_and_middlewares() {
    let (repo, _bare) = setup_repo();
    let mut p = instance(repo.path()).await;
    let meta = p.init(&serde_json::json!({})).await.unwrap();
    assert_eq!(meta.name, "git");
    assert_eq!(meta.version, "0.1.0");
    let names: Vec<_> = p
        .list_middlewares()
        .await
        .unwrap()
        .into_iter()
        .map(|m| m.name)
        .collect();
    for expected in ["repo-context", "latest-tag", "commits", "tag", "push"] {
        assert!(names.contains(&expected.to_string()), "missing {expected}");
    }
}

#[tokio::test]
async fn full_release_flow() {
    let (repo, bare) = setup_repo();
    let dir = repo.path();
    let mut p = instance(dir).await;

    // repo-context
    let r = p
        .execute("repo-context", ctx(dir, "ctx"), &serde_json::json!({}))
        .await
        .unwrap();
    assert!(r.successful);
    let m: std::collections::HashMap<_, _> = r.output.into_iter().collect();
    assert_eq!(m["branch"], serde_json::json!("main"));
    assert_eq!(
        m["remoteUrl"],
        serde_json::json!(bare.path().to_str().unwrap())
    );

    // latest-tag (prefix v -> name 1.0.0), stores SHA in shared state
    let r = p
        .execute(
            "latest-tag",
            ctx(dir, "tag"),
            &serde_json::json!({ "prefix": "v" }),
        )
        .await
        .unwrap();
    assert!(r.successful);
    let m: std::collections::HashMap<_, _> = r.output.into_iter().collect();
    assert_eq!(m["name"], serde_json::json!("1.0.0"));
    assert_eq!(m["fullName"], serde_json::json!("v1.0.0"));

    // commits since the tag (shared context) -> only "fix: second"
    let r = p
        .execute("commits", ctx(dir, "commits"), &serde_json::json!({}))
        .await
        .unwrap();
    assert!(r.successful);
    let m: std::collections::HashMap<_, _> = r.output.into_iter().collect();
    assert_eq!(m["count"], serde_json::json!(1));
    let details = m["details"].as_array().unwrap();
    assert_eq!(details.len(), 1);
    assert_eq!(details[0]["message"], serde_json::json!("fix: second"));

    // tag: create v2.0.0, then re-run -> warning
    let r = p
        .execute(
            "tag",
            ctx(dir, "newtag"),
            &serde_json::json!({ "tagName": "v2.0.0" }),
        )
        .await
        .unwrap();
    assert!(r.successful, "creating v2.0.0");
    let r = p
        .execute(
            "tag",
            ctx(dir, "retag"),
            &serde_json::json!({ "tagName": "v2.0.0" }),
        )
        .await
        .unwrap();
    assert!(r.successful);
    assert!(!r.warnings.is_empty(), "existing tag warns");

    // push to the bare remote
    let r = p
        .execute("push", ctx(dir, "push"), &serde_json::json!({}))
        .await
        .unwrap();
    assert!(r.successful, "push failed: {:?}", r.error_message);

    // the bare remote now has the branch
    let branches = std::process::Command::new("git")
        .args(["branch", "--list"])
        .current_dir(bare.path())
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&branches.stdout).contains("main"));
}
