use crate::config::{Config, PluginSpec};

#[must_use]
pub fn expand_plugins(config: &Config) -> Vec<PluginSpec> {
    config
        .plugins
        .iter()
        .filter(|plugin| plugin.enabled())
        .cloned()
        .collect()
}
