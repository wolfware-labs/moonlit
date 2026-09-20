use crate::plugin::model::PluginMetadata;

mod model;
mod resolver;

pub struct Plugin {
    metadata: PluginMetadata,
}
