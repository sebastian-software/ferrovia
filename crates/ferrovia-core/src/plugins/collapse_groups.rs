use crate::plugins::_collections::{is_animation_elem, is_inheritable_attr};
use crate::style::{collect_stylesheet, compute_style};
use crate::types::{ComputedStyle, XastChild, XastElement, XastRoot};

/// Apply the `collapseGroups` plugin.
///
/// # Errors
///
/// This direct port currently does not return plugin-specific runtime errors.
pub fn apply(root: &mut XastRoot) -> crate::error::Result<()> {
    let stylesheet = collect_stylesheet(root);
    collapse_groups(&mut root.children, Some("#root"), &stylesheet);
    Ok(())
}

fn collapse_groups(
    children: &mut Vec<XastChild>,
    parent_name: Option<&str>,
    stylesheet: &crate::types::Stylesheet,
) {
    let mut index = 0usize;
    while index < children.len() {
        let mut replaced = false;
        if let XastChild::Element(element) = &mut children[index] {
            let name = element.name.clone();
            collapse_groups(&mut element.children, Some(name.as_str()), stylesheet);

            if parent_name == Some("#root") || parent_name == Some("switch") {
                index += 1;
                continue;
            }
            if element.name != "g" || element.children.is_empty() {
                index += 1;
                continue;
            }

            if !element.attributes.is_empty() && element.children.len() == 1 {
                let node_has_filter = element.get_attribute("filter").is_some()
                    || compute_style(stylesheet, element).iter().any(|(name, value)| {
                        name == "filter"
                            && matches!(
                                value,
                                ComputedStyle::Static { value, .. } if !value.is_empty()
                            )
                    });

                let can_try_move = matches!(
                    element.children.first(),
                    Some(XastChild::Element(first_child))
                        if first_child.get_attribute("id").is_none()
                            && !node_has_filter
                            && (element.get_attribute("class").is_none()
                                || first_child.get_attribute("class").is_none())
                            && ((element.get_attribute("clip-path").is_none()
                                && element.get_attribute("mask").is_none())
                                || (first_child.name == "g"
                                    && element.get_attribute("transform").is_none()
                                    && first_child.get_attribute("transform").is_none()))
                );

                if can_try_move {
                    let mut should_abort = false;
                    if let Some(XastChild::Element(first_child)) = element.children.first_mut() {
                        for attribute in element.attributes.clone() {
                            if has_animated_attr(first_child, attribute.name.as_str()) {
                                should_abort = true;
                                break;
                            }

                            if first_child.get_attribute(attribute.name.as_str()).is_none() {
                                first_child.set_attribute(attribute.name.as_str(), attribute.value);
                            } else if attribute.name == "transform" {
                                let value = format!(
                                    "{} {}",
                                    attribute.value,
                                    first_child
                                        .get_attribute("transform")
                                        .unwrap_or_default()
                                );
                                first_child.set_attribute("transform", value);
                            } else if first_child.get_attribute(attribute.name.as_str())
                                == Some("inherit")
                            {
                                first_child
                                    .set_attribute(attribute.name.as_str(), attribute.value);
                            } else if !is_inheritable_attr(attribute.name.as_str())
                                && first_child.get_attribute(attribute.name.as_str())
                                    != Some(attribute.value.as_str())
                            {
                                should_abort = true;
                                break;
                            }
                        }
                    }
                    if !should_abort {
                        element.attributes.clear();
                    }
                }
            }

            if element.attributes.is_empty()
                && !element.children.iter().any(
                    |child| matches!(child, XastChild::Element(node) if is_animation_elem(node.name.as_str())),
                )
            {
                let replacement = std::mem::take(&mut element.children);
                children.remove(index);
                for (offset, child) in replacement.into_iter().enumerate() {
                    children.insert(index + offset, child);
                }
                replaced = true;
            }
        }

        if !replaced {
            index += 1;
        }
    }
}

fn has_animated_attr(node: &XastElement, name: &str) -> bool {
    for child in &node.children {
        if let XastChild::Element(element) = child {
            if is_animation_elem(element.name.as_str())
                && element.get_attribute("attributeName") == Some(name)
            {
                return true;
            }
            if has_animated_attr(element, name) {
                return true;
            }
        }
    }
    false
}
