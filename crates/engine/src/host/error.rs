#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum HostError {
    #[error("failed to instantiate plugin component: {0}")]
    #[diagnostic(code(moonlit::host::instantiate))]
    Instantiate(String),
    #[error("failed to link host interface: {0}")]
    #[diagnostic(code(moonlit::host::link))]
    Link(String),
    #[error("plugin trapped during {op}: {message}")]
    #[diagnostic(code(moonlit::host::trap))]
    Trap { op: String, message: String },
    #[error("plugin returned malformed JSON for {context}: {source}")]
    #[diagnostic(
        code(moonlit::host::bad_json),
        help("The plugin returned invalid JSON; check the plugin's output serialization logic")
    )]
    BadJson {
        context: String,
        source: serde_json::Error,
    },
    #[error(transparent)]
    #[diagnostic(code(moonlit::host::io))]
    Io(#[from] std::io::Error),
}
