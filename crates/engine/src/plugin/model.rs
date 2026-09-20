#[derive(Clone, Debug, PartialEq)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub icon: Option<String>,
}

impl From<crate::wit::PluginMetadata> for PluginMetadata {
    fn from(value: crate::wit::PluginMetadata) -> Self {
        Self {
            name: value.name,
            version: value.version,
            description: value.description,
            icon: value.icon,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MiddlewareInfo {
    pub name: String,
    pub description: String,
    pub input_schema: Option<String>,
    pub output_schema: Option<String>,
}

impl From<crate::wit::MiddlewareInfo> for MiddlewareInfo {
    fn from(value: crate::wit::MiddlewareInfo) -> Self {
        Self {
            name: value.name,
            description: value.description,
            input_schema: value.input_schema,
            output_schema: value.output_schema,
        }
    }
}
