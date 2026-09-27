#[derive(Clone, Debug, PartialEq)]
pub struct MiddlewareInfo {
  pub name: String,
  pub description: String,
  pub input_schema: Option<String>,
  pub output_schema: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MiddlewareResult {
  pub successful: bool,
  pub error_message: Option<String>,
  pub warnings: Vec<String>,
  pub output: Vec<(String, serde_json::Value)>,
}
