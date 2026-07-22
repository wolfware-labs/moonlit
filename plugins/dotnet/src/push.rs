//! `dotnet nuget push` — publish a `.nupkg` to a NuGet source.

use crate::config::DotnetConfig;
use crate::dotnet::{dotnet, exit_phrase, resolve};
use moonlit_plugin_sdk::prelude::*;
use moonlit_plugin_sdk::process::LineHandler;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PushConfig {
    pub package: String,
    pub source: Option<String>,
    pub api_key: Option<String>,
}

fn non_blank(s: &str) -> bool {
    !s.trim().is_empty()
}

#[derive(Default)]
pub struct Push;

impl Middleware for Push {
    const NAME: &'static str = "push";
    const DESCRIPTION: &'static str = "push a .nupkg package to a NuGet source";
    type Config = PushConfig;

    fn execute(&self, ctx: &Context, cfg: PushConfig) -> MiddlewareResult {
        let pkg_path = resolve(ctx.working_dir(), &cfg.package);
        if !pkg_path.is_file() {
            return MiddlewareResult::failure(format!(
                "NuGet package file not found at path: {}",
                pkg_path.display()
            ));
        }

        let plugin = ctx.plugin_config::<DotnetConfig>();
        let source = cfg
            .source
            .as_deref()
            .filter(|s| non_blank(s))
            .unwrap_or(plugin.nuget_source.as_str());
        if !non_blank(source) {
            return MiddlewareResult::failure(
                "NuGet source is not specified in both global and local configuration.",
            );
        }
        let api_key = cfg
            .api_key
            .as_deref()
            .filter(|s| non_blank(s))
            .unwrap_or(plugin.resolved_api_key());
        if !non_blank(api_key) {
            return MiddlewareResult::failure(
                "NuGet API key is not specified in both global and local configuration.",
            );
        }

        let args = vec![
            "nuget".to_string(),
            "push".to_string(),
            cfg.package.clone(),
            "--source".to_string(),
            source.to_string(),
            "--api-key".to_string(),
            api_key.to_string(),
            "--timeout".to_string(),
            "30".to_string(),
        ];

        match dotnet(ctx).args(args).stream(LineHandler::severity()) {
            Ok(o) if o.success() => MiddlewareResult::success(),
            Ok(o) => {
                let combined = format!("{}\n{}", o.stdout(), o.stderr()).to_ascii_lowercase();
                if combined.contains("401")
                    || combined.contains("403")
                    || combined.contains("unauthorized")
                    || combined.contains("forbidden")
                {
                    MiddlewareResult::failure(
                        "Failed to push package: Authentication error. Please check your API key and permissions.",
                    )
                } else {
                    MiddlewareResult::failure(format!(
                        "Failed to push package: {}",
                        exit_phrase(o.exit_code)
                    ))
                }
            }
            Err(e) => MiddlewareResult::failure(format!("Failed to push package: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moonlit_plugin_sdk::process::{OutputChunk, StdioStream};
    use moonlit_plugin_sdk::testing::{run, MockHost};

    fn pkg_dir() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("App.1.0.0.nupkg"), b"pkg").unwrap();
        d
    }
    fn ctx_with<'a>(
        host: &'a MockHost,
        dir: &std::path::Path,
        cfg: &'a DotnetConfig,
    ) -> Context<'a> {
        Context::new(host, dir.to_str().unwrap().into(), "push".into()).with_plugin_config(cfg)
    }
    fn err(text: &str) -> OutputChunk {
        OutputChunk {
            stream: StdioStream::Stderr,
            text: text.to_string(),
        }
    }

    #[test]
    fn pushes_with_source_and_key_from_config() {
        let d = pkg_dir();
        let host = MockHost::new().with_process_result(0, vec![]);
        let plugin = DotnetConfig::default();
        let ctx = ctx_with(&host, d.path(), &plugin);
        let cfg = PushConfig {
            package: "App.1.0.0.nupkg".into(),
            source: Some("https://feed/v3".into()),
            api_key: Some("SECRET".into()),
        };
        assert!(run(&Push, &ctx, cfg).is_success());
        let cmds = host.recorded_commands();
        assert_eq!(
            cmds[0].args,
            vec![
                "nuget",
                "push",
                "App.1.0.0.nupkg",
                "--source",
                "https://feed/v3",
                "--api-key",
                "SECRET",
                "--timeout",
                "30",
            ]
        );
    }

    #[test]
    fn falls_back_to_plugin_config_source_and_key() {
        let d = pkg_dir();
        let host = MockHost::new().with_process_result(0, vec![]);
        let plugin: DotnetConfig = serde_json::from_value(
            serde_json::json!({ "nugetSource": "https://plug", "apiKey": "PKEY" }),
        )
        .unwrap();
        let ctx = ctx_with(&host, d.path(), &plugin);
        let cfg = PushConfig {
            package: "App.1.0.0.nupkg".into(),
            source: None,
            api_key: None,
        };
        assert!(run(&Push, &ctx, cfg).is_success());
        let cmds = host.recorded_commands();
        assert_eq!(cmds[0].args[3], "--source");
        assert_eq!(cmds[0].args[4], "https://plug");
        assert_eq!(cmds[0].args[6], "PKEY");
    }

    #[test]
    fn missing_package_fails() {
        let d = tempfile::tempdir().unwrap();
        let host = MockHost::new();
        let plugin = DotnetConfig::default();
        let ctx = ctx_with(&host, d.path(), &plugin);
        let cfg = PushConfig {
            package: "nope.nupkg".into(),
            source: Some("s".into()),
            api_key: Some("k".into()),
        };
        let w = run(&Push, &ctx, cfg).into_wit();
        assert!(w
            .error_message
            .unwrap()
            .starts_with("NuGet package file not found at path:"));
    }

    #[test]
    fn blank_source_everywhere_fails() {
        let d = pkg_dir();
        let host = MockHost::new();
        let plugin: DotnetConfig =
            serde_json::from_value(serde_json::json!({ "nugetSource": "" })).unwrap();
        let ctx = ctx_with(&host, d.path(), &plugin);
        let cfg = PushConfig {
            package: "App.1.0.0.nupkg".into(),
            source: None,
            api_key: Some("k".into()),
        };
        let w = run(&Push, &ctx, cfg).into_wit();
        assert_eq!(
            w.error_message.as_deref(),
            Some("NuGet source is not specified in both global and local configuration.")
        );
    }

    #[test]
    fn blank_key_everywhere_fails() {
        let d = pkg_dir();
        let host = MockHost::new();
        let plugin = DotnetConfig::default(); // blank keys
        let ctx = ctx_with(&host, d.path(), &plugin);
        let cfg = PushConfig {
            package: "App.1.0.0.nupkg".into(),
            source: Some("s".into()),
            api_key: None,
        };
        let w = run(&Push, &ctx, cfg).into_wit();
        assert_eq!(
            w.error_message.as_deref(),
            Some("NuGet API key is not specified in both global and local configuration.")
        );
    }

    #[test]
    fn http_401_maps_to_auth_error() {
        let d = pkg_dir();
        let host = MockHost::new().with_process_result(
            1,
            vec![err(
                "error: Response status code does not indicate success: 401 (Unauthorized).",
            )],
        );
        let plugin = DotnetConfig::default();
        let ctx = ctx_with(&host, d.path(), &plugin);
        let cfg = PushConfig {
            package: "App.1.0.0.nupkg".into(),
            source: Some("s".into()),
            api_key: Some("k".into()),
        };
        let w = run(&Push, &ctx, cfg).into_wit();
        assert_eq!(
            w.error_message.as_deref(),
            Some("Failed to push package: Authentication error. Please check your API key and permissions.")
        );
    }

    #[test]
    fn other_non_zero_maps_to_generic() {
        let d = pkg_dir();
        let host = MockHost::new().with_process_result(1, vec![err("error: connection reset")]);
        let plugin = DotnetConfig::default();
        let ctx = ctx_with(&host, d.path(), &plugin);
        let cfg = PushConfig {
            package: "App.1.0.0.nupkg".into(),
            source: Some("s".into()),
            api_key: Some("k".into()),
        };
        let w = run(&Push, &ctx, cfg).into_wit();
        assert_eq!(
            w.error_message.as_deref(),
            Some("Failed to push package: Dotnet command failed with exit code 1")
        );
    }
}
