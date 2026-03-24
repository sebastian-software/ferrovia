use crate::plugins::_collections::is_conditional_processing_attr;
use crate::types::{XastChild, XastRoot};

/// Apply the `removeEmptyAttrs` plugin.
///
/// # Errors
///
/// This direct port currently does not return plugin-specific runtime errors.
pub fn apply(root: &mut XastRoot) -> crate::error::Result<()> {
    remove_empty_attrs(&mut root.children);
    Ok(())
}

fn remove_empty_attrs(children: &mut [XastChild]) {
    for child in children {
        if let XastChild::Element(element) = child {
            element.attributes.retain(|attribute| {
                !attribute.value.is_empty()
                    || is_conditional_processing_attr(attribute.name.as_str())
            });
            remove_empty_attrs(&mut element.children);
        }
    }
}
