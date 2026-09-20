mod model;
mod publish;
mod resolver;

use crate::plugin::model::PluginMetadata;

pub struct Plugin {
    metadata: PluginMetadata,
}
