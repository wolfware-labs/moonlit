wit_bindgen::generate!({
    path: "../../crates/pdk/wit",
    world: "plugin",
    generate_all,
});

use crate::moonlit::plugin::host;
use crate::moonlit::plugin::process;
use crate::moonlit::plugin::types::{LogLevel, PermissionKind, PermissionRequest};

struct Component;

static EXEC_ON_LIST: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

const ICON_DATA_URI: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAAC0lEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
const LOG_AND_OUTPUT_INPUT_SCHEMA: &str = r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","title":"LogAndOutputConfig","type":"object","properties":{"name":{"type":"string","description":"config name echoed into the step output"}},"additionalProperties":false}"#;
const LOG_AND_OUTPUT_OUTPUT_SCHEMA: &str = r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","title":"LogAndOutputOutput","type":"object","properties":{"step":{"type":"string","description":"the step name this ran under"},"echoed_config":{"description":"the raw config echoed back"},"cfg_seen":{"description":"the plugin:name config value observed"}},"additionalProperties":false}"#;

impl Guest for Component {
    fn describe() -> PluginMetadata {
        PluginMetadata {
            name: "test-plugin".to_string(),
            version: "0.1.0".to_string(),
            description: "Moonlit host test fixture".to_string(),
            icon: Some(ICON_DATA_URI.to_string()),
            permissions: vec![
                PermissionRequest {
                    kind: PermissionKind::Network,
                    values: vec!["api.example.com".to_string()],
                    reason: "Calls the example API.".to_string(),
                    optional: false,
                },
                PermissionRequest {
                    kind: PermissionKind::Exec,
                    values: vec!["git".to_string()],
                    reason: "Only for the tag middleware.".to_string(),
                    optional: true,
                },
            ],
        }
    }

    fn init(plugin_config: String) -> Result<PluginMetadata, String> {
        if let Some(program) = json_field(&plugin_config, "execOnInit") {
            try_exec(program);
        }
        if plugin_config.contains("\"failInit\":true")
            || plugin_config.contains("\"failInit\": true")
            || plugin_config.contains("\"failInit\":\"true\"")
            || plugin_config.contains("\"failInit\": \"true\"")
        {
            return Err("init failed on request (failInit=true)".to_string());
        }
        *EXEC_ON_LIST.lock().unwrap() = json_field(&plugin_config, "execOnList");
        Ok(Self::describe())
    }

    fn list_middlewares() -> Vec<MiddlewareInfo> {
        if let Some(program) = EXEC_ON_LIST.lock().unwrap().clone() {
            try_exec(program);
        }
        let mw = |name: &str,
                  description: &str,
                  input_schema: Option<String>,
                  output_schema: Option<String>| MiddlewareInfo {
            name: name.to_string(),
            description: description.to_string(),
            input_schema,
            output_schema,
        };
        vec![
            mw(
                "log-and-output",
                "logs + outputs",
                Some(LOG_AND_OUTPUT_INPUT_SCHEMA.to_string()),
                Some(LOG_AND_OUTPUT_OUTPUT_SCHEMA.to_string()),
            ),
            mw("run-process", "process::run", None, None),
            mw("spawn-stream", "process::spawn streaming", None, None),
            mw("http-get", "wasi:http GET", None, None),
            mw("try-exec", "runs config program, ignores errors", None, None),
            mw("try-fetch", "GETs config url, ignores errors", None, None),
            mw("boom", "panics", None, None),
            mw("fail", "returns successful=false", None, None),
            mw("dup-output", "two outputs, same key", None, None),
            mw("sleep", "blocks for config ms", None, None),
            mw(
                "bad-output",
                "successful=true but invalid-JSON output",
                None,
                None,
            ),
        ]
    }

    fn execute(middleware: String, ctx: ReleaseContext, config: String) -> MiddlewareResult {
        match middleware.as_str() {
            "log-and-output" => {
                host::log(
                    LogLevel::Info,
                    &format!("executing in {}", ctx.working_directory),
                );
                host::report_progress("halfway there");
                let cfg_seen =
                    host::get_config("plugin:name").unwrap_or_else(|| "null".to_string());
                MiddlewareResult {
                    successful: true,
                    error_message: None,
                    warnings: vec!["a benign warning".to_string()],
                    output: vec![
                        ("step".to_string(), format!("\"{}\"", ctx.step_name)),
                        ("echoed_config".to_string(), config.clone()),
                        ("cfg_seen".to_string(), cfg_seen),
                    ],
                }
            }
            "run-process" => {
                let cmd = process::Command {
                    program: "echo".to_string(),
                    args: vec!["hello".to_string()],
                    cwd: None,
                    env: vec![],
                    stdin: None,
                };
                match process::run(&cmd) {
                    Ok((code, chunks)) => {
                        let joined: String = chunks
                            .into_iter()
                            .map(|c| c.line)
                            .collect::<Vec<_>>()
                            .join("\n");
                        MiddlewareResult {
                            successful: code == 0,
                            error_message: None,
                            warnings: vec![],
                            output: vec![
                                ("exit_code".to_string(), code.to_string()),
                                ("stdout".to_string(), format!("{joined:?}")),
                            ],
                        }
                    }
                    Err(e) => MiddlewareResult {
                        successful: false,
                        error_message: Some(e),
                        warnings: vec![],
                        output: vec![],
                    },
                }
            }
            "spawn-stream" => {
                let cmd = process::Command {
                    program: "sh".to_string(),
                    args: vec!["-c".to_string(), "echo a; echo b; echo c".to_string()],
                    cwd: None,
                    env: vec![],
                    stdin: None,
                };
                match process::spawn(&cmd) {
                    Ok(child) => {
                        let mut lines = Vec::new();
                        while let Some(chunk) = child.next_line() {
                            lines.push(chunk.line);
                        }
                        let code = child.wait();
                        MiddlewareResult {
                            successful: code == 0,
                            error_message: None,
                            warnings: vec![],
                            output: vec![
                                ("lines".to_string(), format!("{:?}", lines.join(","))),
                                ("exit_code".to_string(), code.to_string()),
                            ],
                        }
                    }
                    Err(e) => MiddlewareResult {
                        successful: false,
                        error_message: Some(e),
                        warnings: vec![],
                        output: vec![],
                    },
                }
            }
            "http-get" => {
                let authority =
                    json_field(&config, "authority").unwrap_or_else(|| "example.com".to_string());
                let path = json_field(&config, "path").unwrap_or_else(|| "/".to_string());
                let scheme = json_field(&config, "scheme").unwrap_or_else(|| "https".to_string());
                match http_get(&scheme, &authority, &path) {
                    Ok(status) => MiddlewareResult {
                        successful: true,
                        error_message: None,
                        warnings: vec![],
                        output: vec![("status".to_string(), status.to_string())],
                    },
                    Err(e) => MiddlewareResult {
                        successful: false,
                        error_message: Some(e),
                        warnings: vec![],
                        output: vec![],
                    },
                }
            }
            "try-exec" => {
                try_exec(json_field(&config, "program").unwrap_or_else(|| "git".to_string()));
                MiddlewareResult {
                    successful: true,
                    error_message: None,
                    warnings: vec![],
                    output: vec![],
                }
            }
            "try-fetch" => {
                let url = json_field(&config, "url").unwrap_or_default();
                let (scheme, rest) = match url.split_once("://") {
                    Some((s, r)) => (s.to_string(), r.to_string()),
                    None => ("https".to_string(), url.clone()),
                };
                let (authority, path) = match rest.split_once('/') {
                    Some((a, p)) => (a.to_string(), format!("/{p}")),
                    None => (rest.clone(), "/".to_string()),
                };
                let _ = http_get(&scheme, &authority, &path);
                MiddlewareResult {
                    successful: true,
                    error_message: None,
                    warnings: vec![],
                    output: vec![],
                }
            }
            "fail" => MiddlewareResult {
                successful: false,
                error_message: Some("intentional failure".to_string()),
                warnings: vec!["fail warning".to_string()],
                output: vec![],
            },
            "dup-output" => MiddlewareResult {
                successful: true,
                error_message: None,
                warnings: vec![],
                output: vec![
                    ("k".to_string(), "\"one\"".to_string()),
                    ("k".to_string(), "\"two\"".to_string()),
                ],
            },
            "sleep" => {
                let ms: u64 = json_field(&config, "ms")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(10_000);
                std::thread::sleep(std::time::Duration::from_millis(ms));
                MiddlewareResult {
                    successful: true,
                    error_message: None,
                    warnings: vec![],
                    output: vec![],
                }
            }
            "bad-output" => MiddlewareResult {
                successful: true,
                error_message: None,
                warnings: vec![],
                output: vec![("k".to_string(), "not valid json".to_string())],
            },
            "boom" => panic!("boom: intentional guest panic to prove trap"),
            other => MiddlewareResult {
                successful: false,
                error_message: Some(format!("unknown middleware: {other}")),
                warnings: vec![],
                output: vec![],
            },
        }
    }
}

fn try_exec(program: String) {
    let cmd = process::Command {
        program,
        args: vec!["--version".to_string()],
        cwd: None,
        env: vec![],
        stdin: None,
    };
    let _ = process::run(&cmd);
}

fn json_field(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let start = json.find(&needle)? + needle.len();
    let rest = &json[start..];
    let colon = rest.find(':')?;
    let after = rest[colon + 1..].trim_start();
    let after = after.strip_prefix('"')?;
    let end = after.find('"')?;
    Some(after[..end].to_string())
}

fn http_get(scheme: &str, authority: &str, path: &str) -> Result<u16, String> {
    use crate::wasi::http::outgoing_handler;
    use crate::wasi::http::types::{Fields, Method, OutgoingRequest, Scheme};

    let s = if scheme == "http" {
        Scheme::Http
    } else {
        Scheme::Https
    };
    let req = OutgoingRequest::new(Fields::new());
    req.set_method(&Method::Get)
        .map_err(|_| "set_method".to_string())?;
    req.set_scheme(Some(&s))
        .map_err(|_| "set_scheme".to_string())?;
    req.set_authority(Some(authority))
        .map_err(|_| "set_authority".to_string())?;
    req.set_path_with_query(Some(path))
        .map_err(|_| "set_path".to_string())?;

    let fut = outgoing_handler::handle(req, None).map_err(|e| format!("handle: {e:?}"))?;
    let pollable = fut.subscribe();
    pollable.block();
    let resp = fut
        .get()
        .ok_or_else(|| "future not ready".to_string())?
        .map_err(|_| "future already taken".to_string())?
        .map_err(|e| format!("response error: {e:?}"))?;
    Ok(resp.status())
}

export!(Component);
