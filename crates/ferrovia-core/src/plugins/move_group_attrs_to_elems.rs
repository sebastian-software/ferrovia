use crate::plugins::_collections::{is_path_elem, is_reference_prop};
use crate::svgo::tools::includes_url_reference;
use crate::types::{XastChild, XastRoot};

/// Apply the `moveGroupAttrsToElems` plugin.
///
/// # Errors
///
/// This direct port currently does not return plugin-specific runtime errors.
pub fn apply(root: &mut XastRoot) -> crate::error::Result<()> {
    move_group_attrs_to_elems(&mut root.children);
    Ok(())
}

fn move_group_attrs_to_elems(children: &mut [XastChild]) {
    for child in children {
        if let XastChild::Element(element) = child {
            if element.name == "g"
                && !element.children.is_empty()
                && element.get_attribute("transform").is_some()
                && !element.attributes.iter().any(|attribute| {
                    is_reference_prop(attribute.name.as_str())
                        && includes_url_reference(attribute.value.as_str())
                })
                && element.children.iter().all(|child| {
                    matches!(
                        child,
                        XastChild::Element(node)
                            if (is_path_elem(node.name.as_str())
                                || node.name == "g"
                                || node.name == "text")
                                && node.get_attribute("id").is_none()
                    )
                })
            {
                let transform = element
                    .get_attribute("transform")
                    .map(str::to_string)
                    .unwrap_or_default();
                for child in &mut element.children {
                    if let XastChild::Element(node) = child {
                        if let Some(existing) = node.get_attribute("transform") {
                            node.set_attribute("transform", format!("{transform} {existing}"));
                        } else {
                            node.set_attribute("transform", transform.clone());
                        }
                    }
                }
                element.remove_attribute("transform");
            }

            move_group_attrs_to_elems(&mut element.children);
        }
    }
}
