use crate::plugins::_collections::{is_inheritable_attr, is_path_elem};
use crate::types::{XastChild, XastRoot};

/// Apply the `moveElemsAttrsToGroup` plugin.
///
/// # Errors
///
/// This direct port currently does not return plugin-specific runtime errors.
pub fn apply(root: &mut XastRoot) -> crate::error::Result<()> {
    let deoptimized_with_styles = has_style_element(&root.children);
    move_elems_attrs_to_group(&mut root.children, deoptimized_with_styles);
    Ok(())
}

fn has_style_element(children: &[XastChild]) -> bool {
    for child in children {
        if let XastChild::Element(element) = child
            && (element.name == "style" || has_style_element(&element.children))
        {
            return true;
        }
    }
    false
}

fn move_elems_attrs_to_group(children: &mut [XastChild], deoptimized_with_styles: bool) {
    for child in children {
        if let XastChild::Element(element) = child {
            move_elems_attrs_to_group(&mut element.children, deoptimized_with_styles);

            if element.name != "g" || element.children.len() <= 1 || deoptimized_with_styles {
                continue;
            }

            let mut common_attributes = Vec::<(String, String)>::new();
            let mut initial = true;
            let mut every_child_is_path = true;
            for child in &element.children {
                if let XastChild::Element(node) = child {
                    if !is_path_elem(node.name.as_str()) {
                        every_child_is_path = false;
                    }

                    if initial {
                        initial = false;
                        for attribute in &node.attributes {
                            if is_inheritable_attr(attribute.name.as_str()) {
                                common_attributes
                                    .push((attribute.name.clone(), attribute.value.clone()));
                            }
                        }
                    } else {
                        common_attributes.retain(|(name, value)| {
                            node.get_attribute(name.as_str()) == Some(value.as_str())
                        });
                    }
                }
            }

            if element.get_attribute("filter").is_some()
                || element.get_attribute("clip-path").is_some()
                || element.get_attribute("mask").is_some()
            {
                common_attributes.retain(|(name, _)| name != "transform");
            }

            if every_child_is_path {
                common_attributes.retain(|(name, _)| name != "transform");
            }

            for (name, value) in &common_attributes {
                if name == "transform" {
                    if let Some(group_transform) = element.get_attribute("transform") {
                        element.set_attribute("transform", format!("{group_transform} {value}"));
                    } else {
                        element.set_attribute("transform", value.clone());
                    }
                } else {
                    element.set_attribute(name.as_str(), value.clone());
                }
            }

            for child in &mut element.children {
                if let XastChild::Element(node) = child {
                    for (name, _) in &common_attributes {
                        node.remove_attribute(name.as_str());
                    }
                }
            }
        }
    }
}
