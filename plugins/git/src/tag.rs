//! `git tag` — create a lightweight or annotated tag (idempotent on existing).

use crate::shared::{ensure_repo, git};
use moonlit_plugin_sdk::prelude::*;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct TagConfig {
    tag_name: String,
    message: Option<String>,
}

#[derive(Default)]
pub struct Tag;

impl Middleware for Tag {
    const NAME: &'static str = "tag";
    const DESCRIPTION: &'static str = "create a lightweight or annotated tag";
    type Config = TagConfig;

    fn execute(&self, ctx: &Context, cfg: TagConfig) -> MiddlewareResult {
        if cfg.tag_name.trim().is_empty() {
            return MiddlewareResult::failure("Tag name cannot be empty.");
        }
        if let Err(f) = ensure_repo(ctx) {
            return f;
        }
        // `--end-of-options` before the tag name keeps a `-`-leading name (e.g. `-d`)
        // from being parsed as a git flag; git then rejects it as an invalid tag name.
        match git(ctx)
            .arg("tag")
            .arg("-l")
            .arg("--end-of-options")
            .arg(&cfg.tag_name)
            .run()
        {
            Ok(o) if o.success() && !o.stdout().trim().is_empty() => {
                return MiddlewareResult::success()
                    .with_warning(format!("Tag '{}' already exists.", cfg.tag_name));
            }
            Ok(_) => {}
            Err(e) => return MiddlewareResult::failure(e),
        }
        let created = match cfg.message.as_deref().filter(|m| !m.is_empty()) {
            // Options (`-a`, `-m <msg>`) precede `--end-of-options`; the tag name follows it.
            Some(m) => git(ctx)
                .arg("tag")
                .arg("-a")
                .arg("-m")
                .arg(m)
                .arg("--end-of-options")
                .arg(&cfg.tag_name)
                .run(),
            None => git(ctx)
                .arg("tag")
                .arg("--end-of-options")
                .arg(&cfg.tag_name)
                .run(),
        };
        match created {
            Ok(o) if o.success() => MiddlewareResult::success(),
            Ok(o) => MiddlewareResult::failure(format!(
                "Git command failed with exit code {}",
                o.exit_code
            )),
            Err(e) => MiddlewareResult::failure(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_plugin_sdk::process::{OutputChunk, StdioStream};
    use moonlit_plugin_sdk::testing::{run, MockHost};

    fn out(text: &str) -> OutputChunk {
        OutputChunk {
            stream: StdioStream::Stdout,
            text: text.to_string(),
        }
    }

    #[test]
    fn blank_name_fails_before_touching_git() {
        let host = MockHost::new();
        let ctx = Context::new(&host, "/repo".into(), "s".into());
        let cfg = TagConfig {
            tag_name: "   ".to_string(),
            ..Default::default()
        };
        let w = run(&Tag, &ctx, cfg).into_wit();
        assert!(!w.successful);
        assert_eq!(
            w.error_message.as_deref(),
            Some("Tag name cannot be empty.")
        );
        assert!(host.recorded_commands().is_empty());
    }

    #[test]
    fn existing_tag_warns_and_succeeds() {
        let host = MockHost::new()
            .with_process_result(0, vec![out(".git")]) // ensure_repo
            .with_process_result(0, vec![out("v1.0.0")]); // tag -l finds it
        let ctx = Context::new(&host, "/repo".into(), "s".into());
        let cfg = TagConfig {
            tag_name: "v1.0.0".to_string(),
            ..Default::default()
        };
        let result = run(&Tag, &ctx, cfg);
        assert!(result.is_success());
        assert_eq!(
            result.warnings(),
            &["Tag 'v1.0.0' already exists.".to_string()]
        );
    }

    #[test]
    fn lightweight_tag_when_no_message() {
        let host = MockHost::new()
            .with_process_result(0, vec![out(".git")]) // ensure_repo
            .with_process_result(0, vec![]) // tag -l: not found
            .with_process_result(0, vec![]); // tag <name>
        let ctx = Context::new(&host, "/repo".into(), "s".into());
        let cfg = TagConfig {
            tag_name: "v2.0.0".to_string(),
            ..Default::default()
        };
        assert!(run(&Tag, &ctx, cfg).is_success());
        let cmds = host.recorded_commands();
        assert_eq!(cmds[2].args, vec!["tag", "--end-of-options", "v2.0.0"]);
    }

    #[test]
    fn annotated_tag_when_message_present() {
        let host = MockHost::new()
            .with_process_result(0, vec![out(".git")])
            .with_process_result(0, vec![]) // tag -l: not found
            .with_process_result(0, vec![]); // tag -a ...
        let ctx = Context::new(&host, "/repo".into(), "s".into());
        let cfg = TagConfig {
            tag_name: "v2.0.0".to_string(),
            message: Some("release 2.0.0".to_string()),
        };
        assert!(run(&Tag, &ctx, cfg).is_success());
        let cmds = host.recorded_commands();
        assert_eq!(
            cmds[2].args,
            vec![
                "tag",
                "-a",
                "-m",
                "release 2.0.0",
                "--end-of-options",
                "v2.0.0"
            ]
        );
    }

    #[test]
    fn dash_leading_tag_name_is_guarded_by_end_of_options() {
        // A config value like "-d" must reach git as a positional (after the
        // end-of-options marker), never as the `-d`/`--delete` flag.
        let host = MockHost::new()
            .with_process_result(0, vec![out(".git")]) // ensure_repo
            .with_process_result(0, vec![]) // tag -l: not found
            .with_process_result(0, vec![]); // tag <name>
        let ctx = Context::new(&host, "/repo".into(), "s".into());
        let cfg = TagConfig {
            tag_name: "-d".to_string(),
            ..Default::default()
        };
        assert!(run(&Tag, &ctx, cfg).is_success());
        let cmds = host.recorded_commands();
        assert_eq!(cmds[1].args, vec!["tag", "-l", "--end-of-options", "-d"]);
        assert_eq!(cmds[2].args, vec!["tag", "--end-of-options", "-d"]);
    }
}
