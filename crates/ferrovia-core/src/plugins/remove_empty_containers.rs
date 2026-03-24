use std::collections::HashSet;

use crate::plugins::_collections::is_container_elem;
use crate::style::{collect_stylesheet, compute_style};
use crate::svgo::tools::find_references;
use crate::types::{ComputedStyle, XastChild, XastElement, XastRoot};

/// Apply the `removeEmptyContainers` plugin.
///
/// # Errors
///
/// This direct port currently does not return plugin-specific runtime errors.
pub fn apply(root: &mut XastRoot) -> crate::error::Result<()> {
    let stylesheet = collect_stylesheet(root);
    let mut removed_ids = HashSet::<String>::new();
    remove_empty_containers(&mut root.children, None, &stylesheet, &mut removed_ids);
    if !removed_ids.is_empty() {
        remove_use_references(&mut root.children, &removed_ids);
    }
    Ok(())
}

fn remove_empty_containers(
    children: &mut Vec<XastChild>,
    parent_name: Option<&str>,
    stylesheet: &crate::types::Stylesheet,
    removed_ids: &mut HashSet<String>,
) {
    let mut index = 0usize;
    while index < children.len() {
        let mut removed = false;
        if let XastChild::Element(element) = &mut children[index] {
            remove_empty_containers(
                &mut element.children,
                Some(element.name.as_str()),
                stylesheet,
                removed_ids,
            );
            if should_remove(element, parent_name, stylesheet) {
                if let Some(id) = element.get_attribute("id") {
                    removed_ids.insert(id.to_string());
                }
                children.remove(index);
                removed = true;
            }
        }
        if !removed {
            index += 1;
        }
    }
}

fn should_remove(
    element: &XastElement,
    parent_name: Option<&str>,
    stylesheet: &crate::types::Stylesheet,
) -> bool {
    if element.name == "svg"
        || !is_container_elem(element.name.as_str())
        || !element.children.is_empty()
    {
        return false;
    }
    if element.name == "pattern" && !element.attributes.is_empty() {
        return false;
    }
    if element.name == "mask" && element.get_attribute("id").is_some() {
        return false;
    }
    if parent_name == Some("switch") {
        return false;
    }
    if element.name == "g"
        && (element.get_attribute("filter").is_some() || has_filter_style(stylesheet, element))
    {
        return false;
    }
    true
}

fn has_filter_style(stylesheet: &crate::types::Stylesheet, element: &XastElement) -> bool {
    compute_style(stylesheet, element)
        .iter()
        .any(|(name, value)| {
            name == "filter"
                && matches!(
                    value,
                    ComputedStyle::Static { value, .. } if !value.is_empty()
                )
        })
}

fn remove_use_references(children: &mut Vec<XastChild>, removed_ids: &HashSet<String>) {
    let mut index = 0usize;
    while index < children.len() {
        let mut removed = false;
        if let XastChild::Element(element) = &mut children[index] {
            if element.name == "use" && references_removed_id(element, removed_ids) {
                children.remove(index);
                removed = true;
            } else {
                remove_use_references(&mut element.children, removed_ids);
            }
        }
        if !removed {
            index += 1;
        }
    }
}

fn references_removed_id(element: &XastElement, removed_ids: &HashSet<String>) -> bool {
    element.attributes.iter().any(|attribute| {
        find_references(attribute.name.as_str(), attribute.value.as_str())
            .into_iter()
            .any(|id| removed_ids.contains(id.as_str()))
    })
}
