use crate::config::PluginSpec;
use crate::error::{FerroviaError, Result};
use crate::types::XastRoot;

pub mod _collections;
pub mod _path;
pub mod _transforms;
pub mod apply_transforms;
pub mod remove_attributes_by_selector;
pub mod remove_comments;
pub mod remove_doctype;
pub mod remove_metadata;
pub mod remove_title;
pub mod remove_xml_proc_inst;

/// Apply one configured plugin to the xast root.
///
/// # Errors
///
/// Returns an error when the requested plugin has not yet been ported in the
/// current rewrite stage.
pub fn apply_plugin(root: &mut XastRoot, plugin: &PluginSpec) -> Result<()> {
    match plugin.name() {
        "removeComments" => remove_comments::apply(root, plugin.params()),
        "removeAttributesBySelector" => remove_attributes_by_selector::apply(root, plugin.params()),
        "removeDoctype" => remove_doctype::apply(root),
        "removeMetadata" => remove_metadata::apply(root),
        "removeTitle" => remove_title::apply(root),
        "removeXMLProcInst" => remove_xml_proc_inst::apply(root),
        other => Err(FerroviaError::UnsupportedPlugin(other.to_string())),
    }
}
