use crate::wit::moonlit::plugin::types::LogLevel as WitLogLevel;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<WitLogLevel> for LogLevel {
    fn from(value: WitLogLevel) -> Self {
        match value {
            WitLogLevel::Debug => LogLevel::Debug,
            WitLogLevel::Error => LogLevel::Error,
            WitLogLevel::Info => LogLevel::Info,
            WitLogLevel::Warn => LogLevel::Warn,
            WitLogLevel::Trace => LogLevel::Trace,
        }
    }
}
