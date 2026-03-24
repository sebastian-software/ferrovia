use serde_json::Value;

use crate::error::Result;
use crate::types::{XastChild, XastRoot};
use crate::xast::query_selector_all;

/// Apply the `removeAttributesBySelector` plugin.
///
/// # Errors
///
/// This direct port currently does not return plugin-specific runtime errors.
pub fn apply(root: &mut XastRoot, params: Option<&Value>) -> Result<()> {
    let Some(params) = params else {
        return Ok(());
    };

    let selectors = params
        .get("selectors")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| vec![params.clone()]);

    for selector_entry in selectors {
        let Some(selector) = selector_entry.get("selector").and_then(Value::as_str) else {
            continue;
        };
        let Some(attributes_value) = selector_entry.get("attributes") else {
            continue;
        };
        let matched_paths = collect_match_paths(root, selector);
        for path in matched_paths {
            if let Some(XastChild::Element(element)) = get_child_mut(root, &path) {
                if let Some(attribute) = attributes_value.as_str() {
                    element.remove_attribute(attribute);
                } else if let Some(attributes) = attributes_value.as_array() {
                    for attribute in attributes {
                        if let Some(attribute) = attribute.as_str() {
                            element.remove_attribute(attribute);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn collect_match_paths(root: &XastRoot, selector: &str) -> Vec<Vec<usize>> {
    let matches = query_selector_all(root, selector);
    let mut paths = Vec::<Vec<usize>>::new();
    for matched in matches {
        if let Some(path) = find_path(&root.children, matched, &mut Vec::new()) {
            paths.push(path);
        }
    }
    paths
}

fn find_path(
    children: &[XastChild],
    target: &XastChild,
    prefix: &mut Vec<usize>,
) -> Option<Vec<usize>> {
    for (index, child) in children.iter().enumerate() {
        prefix.push(index);
        if std::ptr::eq(child, target) {
            return Some(prefix.clone());
        }
        if let XastChild::Element(element) = child
            && let Some(path) = find_path(&element.children, target, prefix)
        {
            return Some(path);
        }
        prefix.pop();
    }
    None
}

fn get_child_mut<'a>(root: &'a mut XastRoot, path: &[usize]) -> Option<&'a mut XastChild> {
    let (first, rest) = path.split_first()?;
    let child = root.children.get_mut(*first)?;
    get_child_mut_from_child(child, rest)
}

fn get_child_mut_from_child<'a>(
    child: &'a mut XastChild,
    path: &[usize],
) -> Option<&'a mut XastChild> {
    if path.is_empty() {
        return Some(child);
    }
    let XastChild::Element(element) = child else {
        return None;
    };
    let (first, rest) = path.split_first()?;
    let next = element.children.get_mut(*first)?;
    get_child_mut_from_child(next, rest)
}
