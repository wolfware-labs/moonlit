#[derive(Clone, Debug, PartialEq)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub icon: Option<String>,
}


#[derive(Clone, Debug, PartialEq)]
pub struct MiddlewareInfo {
    pub name: String,
    pub description: String,
    pub input_schema: Option<String>,
    pub output_schema: Option<String>,
}