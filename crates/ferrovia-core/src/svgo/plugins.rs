use crate::config::PluginSpec;

#[must_use]
pub fn plugin_name(plugin: &PluginSpec) -> &str {
    plugin.name()
}
