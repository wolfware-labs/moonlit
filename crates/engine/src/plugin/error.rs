#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum PluginError {
    #[error("failed to load plugin component: {0}")]
    #[diagnostic(code(moonlit::plugin::load))]
    Load(String),
    #[error("failed to compile plugin component: {0}")]
    #[diagnostic(code(moonlit::plugin::compile))]
    Compile(String),
    #[error("failed to instantiate plugin component: {0}")]
    #[diagnostic(code(moonlit::plugin::instantiate))]
    Instantiate(String),
    #[error("failed to link host interface: {0}")]
    #[diagnostic(code(moonlit::plugin::link))]
    Link(String),
    #[error("plugin trapped during {op}: {message}")]
    #[diagnostic(code(moonlit::plugin::trap))]
    Trap { op: String, message: String },
    #[error("plugin returned malformed JSON for {context}: {source}")]
    #[diagnostic(
        code(moonlit::plugin::bad_json),
        help("The plugin returned invalid JSON; check the plugin's output serialization logic")
    )]
    BadJson {
        context: String,
        source: serde_json::Error,
    },
    #[error(transparent)]
    #[diagnostic(code(moonlit::plugin::io))]
    Io(#[from] std::io::Error),
}
