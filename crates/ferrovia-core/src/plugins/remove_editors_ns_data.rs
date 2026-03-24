use serde_json::Value;

use crate::plugins::_collections::is_editor_namespace;
use crate::types::{XastChild, XastRoot};

/// Apply the `removeEditorsNSData` plugin.
///
/// # Errors
///
/// This direct port currently does not return plugin-specific runtime errors.
pub fn apply(root: &mut XastRoot, params: Option<&Value>) -> crate::error::Result<()> {
    let mut namespaces = params
        .and_then(|value| value.get("additionalNamespaces"))
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        });
    let mut prefixes = Vec::<String>::new();
    remove_editors_ns_data(&mut root.children, &mut prefixes, &mut namespaces);
    Ok(())
}

fn remove_editors_ns_data(
    children: &mut Vec<XastChild>,
    prefixes: &mut Vec<String>,
    additional_namespaces: &mut [String],
) {
    let mut index = 0usize;
    while index < children.len() {
        let mut removed = false;
        if let XastChild::Element(element) = &mut children[index] {
            if element.name == "svg" {
                let mut attribute_index = 0usize;
                while attribute_index < element.attributes.len() {
                    let attribute = &element.attributes[attribute_index];
                    let is_editor_ns = attribute.name.starts_with("xmlns:")
                        && (is_editor_namespace(attribute.value.as_str())
                            || additional_namespaces.contains(&attribute.value));
                    if is_editor_ns {
                        prefixes.push(attribute.name["xmlns:".len()..].to_string());
                        element.attributes.remove(attribute_index);
                    } else {
                        attribute_index += 1;
                    }
                }
            }

            let mut attribute_index = 0usize;
            while attribute_index < element.attributes.len() {
                let should_remove = element.attributes[attribute_index]
                    .name
                    .split_once(':')
                    .is_some_and(|(prefix, _)| prefixes.iter().any(|item| item == prefix));
                if should_remove {
                    element.attributes.remove(attribute_index);
                } else {
                    attribute_index += 1;
                }
            }

            let should_remove_element = element
                .name
                .split_once(':')
                .is_some_and(|(prefix, _)| prefixes.iter().any(|item| item == prefix));
            if should_remove_element {
                children.remove(index);
                removed = true;
            } else {
                remove_editors_ns_data(&mut element.children, prefixes, additional_namespaces);
            }
        }

        if !removed {
            index += 1;
        }
    }
}
