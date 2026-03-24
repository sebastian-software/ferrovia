use crate::config::PluginSpec;
use crate::error::{FerroviaError, Result};
use crate::types::XastRoot;

pub mod _collections;
pub mod _path;
pub mod _transforms;
pub mod apply_transforms;
pub mod remove_attributes_by_selector;
pub mod remove_attrs;
pub mod remove_comments;
pub mod remove_deprecated_attrs;
pub mod remove_desc;
pub mod remove_dimensions;
pub mod remove_doctype;
pub mod remove_editors_ns_data;
pub mod remove_elements_by_attr;
pub mod remove_empty_attrs;
pub mod remove_empty_containers;
pub mod remove_empty_text;
pub mod remove_metadata;
pub mod remove_raster_images;
pub mod remove_scripts;
pub mod remove_style_element;
pub mod remove_title;
pub mod remove_xml_proc_inst;
pub mod remove_xmlns;

/// Apply one configured plugin to the xast root.
///
/// # Errors
///
/// Returns an error when the requested plugin has not yet been ported in the
/// current rewrite stage.
pub fn apply_plugin(root: &mut XastRoot, plugin: &PluginSpec) -> Result<()> {
    match plugin.name() {
        "removeComments" => remove_comments::apply(root, plugin.params()),
        "removeDesc" => remove_desc::apply(root, plugin.params()),
        "removeDeprecatedAttrs" => remove_deprecated_attrs::apply(root, plugin.params()),
        "removeDimensions" => remove_dimensions::apply(root),
        "removeAttributesBySelector" => remove_attributes_by_selector::apply(root, plugin.params()),
        "removeAttrs" => remove_attrs::apply(root, plugin.params()),
        "removeDoctype" => remove_doctype::apply(root),
        "removeEditorsNSData" => remove_editors_ns_data::apply(root, plugin.params()),
        "removeElementsByAttr" => remove_elements_by_attr::apply(root, plugin.params()),
        "removeEmptyAttrs" => remove_empty_attrs::apply(root),
        "removeEmptyContainers" => remove_empty_containers::apply(root),
        "removeEmptyText" => remove_empty_text::apply(root, plugin.params()),
        "removeMetadata" => remove_metadata::apply(root),
        "removeRasterImages" => remove_raster_images::apply(root),
        "removeScripts" => remove_scripts::apply(root),
        "removeStyleElement" => remove_style_element::apply(root),
        "removeTitle" => remove_title::apply(root),
        "removeXMLNS" => remove_xmlns::apply(root),
        "removeXMLProcInst" => remove_xml_proc_inst::apply(root),
        other => Err(FerroviaError::UnsupportedPlugin(other.to_string())),
    }
}
