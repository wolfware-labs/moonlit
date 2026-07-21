//! Black-box: run the git plugin through `moonlit run` against a temp repo.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use tempfile::tempdir;

fn git_wasm_url() -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../engine/tests/fixtures/git.wasm")
        .canonicalize()
        .expect("git.wasm fixture exists (regenerate via plugins/git/README.md)");
    format!("file://{}", p.display())
}

fn git(dir: &Path, args: &[&str]) {
    let status = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git runs");
    assert!(status.success(), "git {args:?} failed");
}

fn setup(dir: &Path) {
    let bare = dir.join("bare.git");
    fs::create_dir(&bare).unwrap();
    git(&bare, &["init", "--bare"]);
    git(dir, &["init", "-b", "main"]);
    git(dir, &["config", "user.email", "t@example.com"]);
    git(dir, &["config", "user.name", "Test"]);
    git(dir, &["remote", "add", "origin", bare.to_str().unwrap()]);
    fs::write(dir.join("a.txt"), "1").unwrap();
    git(dir, &["add", "a.txt"]);
    git(dir, &["commit", "-m", "feat: first"]);
    git(dir, &["tag", "v1.0.0"]);
}

#[test]
fn moonlit_run_drives_git_plugin() {
    let dir = tempdir().unwrap();
    setup(dir.path());
    let yaml = format!(
        "name: demo\n\
         plugins:\n\
         \x20 - name: git\n\
         \x20   url: {url}\n\
         \x20   permissions:\n\
         \x20     exec: [git]\n\
         \x20     filesystem: read-write\n\
         stages:\n\
         \x20 release:\n\
         \x20   - name: ctx\n\
         \x20     run: git.repo-context\n\
         \x20   - name: tag\n\
         \x20     run: git.latest-tag\n\
         \x20     config:\n\
         \x20       prefix: v\n\
         \x20   - name: commits\n\
         \x20     run: git.commits\n",
        url = git_wasm_url()
    );
    fs::write(dir.path().join("release.yml"), yaml).unwrap();

    Command::cargo_bin("moonlit")
        .unwrap()
        .args(["run", "--output", "plain", "-w"])
        .arg(dir.path())
        .assert()
        .success();
}
