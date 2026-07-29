//! `github write-variables` — append key=value (heredoc for multiline) to the
//! files at $GITHUB_OUTPUT / $GITHUB_ENV. Those files are outside the wasm fs
//! sandbox, so the append runs host-side through the process capability.

use std::collections::BTreeMap;

use moonlit_sdk::prelude::*;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct WriteVariablesConfig {
    output: BTreeMap<String, String>,
    environment: BTreeMap<String, String>,
}

fn render_lines(map: &BTreeMap<String, String>) -> Result<String, String> {
    let mut s = String::new();
    for (k, v) in map {
        if v.contains('\n') {
            // Refuse heredoc delimiter smuggling: a value line equal to the
            // delimiter would close the heredoc early, letting the remaining
            // lines inject arbitrary $GITHUB_OUTPUT/$GITHUB_ENV directives into
            // the runner. Reject rather than silently truncate.
            if v.lines().any(|l| l.trim() == "EOF") {
                return Err(format!(
                    "Refusing to write '{k}': value contains a line equal to the heredoc delimiter 'EOF'."
                ));
            }
            s.push_str(&format!("{k}<<EOF\n{v}\nEOF\n"));
        } else {
            s.push_str(&format!("{k}={v}\n"));
        }
    }
    Ok(s)
}

fn append(
    ctx: &Context,
    var: &str,
    map: &BTreeMap<String, String>,
) -> Result<(), MiddlewareResult> {
    let path = match ctx.env().var(var) {
        Some(p) if !p.is_empty() => p,
        _ => return Err(MiddlewareResult::failure(format!("{var} is not set."))),
    };
    let content = match render_lines(map) {
        Ok(c) => c,
        Err(e) => return Err(MiddlewareResult::failure(e)),
    };
    match ctx
        .command("sh")
        .arg("-c")
        .arg(r#"cat >> "$1""#)
        .arg("sh") // $0
        .arg(&path) // $1
        .stdin(content)
        .run()
    {
        Ok(o) if o.success() => Ok(()),
        Ok(o) => Err(MiddlewareResult::failure(format!(
            "Failed to write {var} (exit code {}).",
            o.exit_code
        ))),
        Err(e) => Err(MiddlewareResult::failure(e)),
    }
}

#[derive(Default)]
pub struct WriteVariables;

impl Middleware for WriteVariables {
    const NAME: &'static str = "write-variables";
    const DESCRIPTION: &'static str = "append step outputs / env to the GitHub Actions files";
    type Config = WriteVariablesConfig;

    fn execute(&self, ctx: &Context, cfg: WriteVariablesConfig) -> MiddlewareResult {
        if !cfg.output.is_empty() {
            if let Err(f) = append(ctx, "GITHUB_OUTPUT", &cfg.output) {
                return f;
            }
        }
        if !cfg.environment.is_empty() {
            if let Err(f) = append(ctx, "GITHUB_ENV", &cfg.environment) {
                return f;
            }
        }
        MiddlewareResult::success()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_sdk::process::{OutputChunk, StdioStream};
    use moonlit_sdk::testing::{run, MockHost};

    fn cfg_out(pairs: &[(&str, &str)]) -> WriteVariablesConfig {
        WriteVariablesConfig {
            output: pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            environment: BTreeMap::new(),
        }
    }

    #[test]
    fn unset_output_var_fails_with_exact_message() {
        let host = MockHost::new(); // no GITHUB_OUTPUT env
        let ctx = Context::new(&host, "/w".into(), "s".into());
        let w = run(&WriteVariables, &ctx, cfg_out(&[("k", "v")])).into_wit();
        assert!(!w.successful);
        assert_eq!(
            w.error_message.as_deref(),
            Some("GITHUB_OUTPUT is not set.")
        );
    }

    #[test]
    fn appends_single_line_via_sh_stdin() {
        let host = MockHost::new()
            .with_env("GITHUB_OUTPUT", "/tmp/out")
            .with_process_result(
                0,
                vec![OutputChunk {
                    stream: StdioStream::Stdout,
                    text: "".into(),
                }],
            );
        let ctx = Context::new(&host, "/w".into(), "s".into());
        let w = run(&WriteVariables, &ctx, cfg_out(&[("version", "1.2.3")])).into_wit();
        assert!(w.successful);
        let cmds = host.recorded_commands();
        assert_eq!(cmds[0].program, "sh");
        assert_eq!(cmds[0].args, vec!["-c", "cat >> \"$1\"", "sh", "/tmp/out"]);
        assert_eq!(cmds[0].stdin.as_deref(), Some("version=1.2.3\n"));
    }

    #[test]
    fn multiline_value_uses_heredoc() {
        let host = MockHost::new()
            .with_env("GITHUB_OUTPUT", "/tmp/out")
            .with_process_result(0, vec![]);
        let ctx = Context::new(&host, "/w".into(), "s".into());
        let w = run(&WriteVariables, &ctx, cfg_out(&[("notes", "line1\nline2")])).into_wit();
        assert!(w.successful);
        let cmds = host.recorded_commands();
        assert_eq!(
            cmds[0].stdin.as_deref(),
            Some("notes<<EOF\nline1\nline2\nEOF\n")
        );
    }

    #[test]
    fn value_with_eof_delimiter_line_is_refused() {
        // A multiline value whose content contains a bare `EOF` line would close
        // the heredoc early and smuggle `PATH=/evil` in as a new directive.
        let host = MockHost::new().with_env("GITHUB_OUTPUT", "/tmp/out");
        let ctx = Context::new(&host, "/w".into(), "s".into());
        let w = run(
            &WriteVariables,
            &ctx,
            cfg_out(&[("notes", "safe\nEOF\nPATH=/evil")]),
        )
        .into_wit();
        assert!(!w.successful);
        assert_eq!(
            w.error_message.as_deref(),
            Some("Refusing to write 'notes': value contains a line equal to the heredoc delimiter 'EOF'.")
        );
        // Refused before any host command ran.
        assert!(host.recorded_commands().is_empty());
    }
}
