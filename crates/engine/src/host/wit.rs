mod raw {
    
}

pub use raw::*;
// 
// #[derive(Clone, Debug, PartialEq)]
// pub struct ReleaseContext {
//     pub working_directory: String,
//     pub step_name: String,
// }
// 
// pub fn json_str_to_value(s: &str, context: &str) -> Result<serde_json::Value, HostError> {
//     serde_json::from_str(s).map_err(|source| HostError::BadJson {
//         context: context.to_string(),
//         source,
//     })
// }
// 
// pub fn log_level(l: raw::LogLevel) -> LogLevel {
//     match l {
//         raw::LogLevel::Trace => LogLevel::Trace,
//         raw::LogLevel::Debug => LogLevel::Debug,
//         raw::LogLevel::Info => LogLevel::Info,
//         raw::LogLevel::Warn => LogLevel::Warn,
//         raw::LogLevel::Error => LogLevel::Error,
//     }
// }
// 
// pub fn release_context_to_raw(ctx: &ReleaseContext) -> raw::ReleaseContext {
//     raw::ReleaseContext {
//         working_directory: ctx.working_directory.clone(),
//         step_name: ctx.step_name.clone(),
//     }
// }
// 
// pub fn middleware_result(r: raw::MiddlewareResult) -> Result<MiddlewareResult, HostError> {
//     let mut output = Vec::with_capacity(r.output.len());
//     for (k, json) in r.output {
//         let value = json_str_to_value(&json, &format!("output key '{k}'"))?;
//         output.push((k, value));
//     }
//     Ok(MiddlewareResult {
//         successful: r.successful,
//         error_message: r.error_message,
//         warnings: r.warnings,
//         output,
//     })
// }
