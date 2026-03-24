use std::collections::{HashMap, HashSet};

use serde_json::Value;

use crate::svgo::tools::{find_references, has_scripts};
use crate::types::{XastChild, XastElement, XastRoot};

const GENERATE_ID_CHARS: &[char] = &[
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r',
    's', 't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J',
    'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
];

#[derive(Clone)]
struct Reference {
    path: Vec<usize>,
    name: String,
}

/// Apply the `cleanupIds` plugin.
///
/// # Errors
///
/// This direct port currently does not return plugin-specific runtime errors.
pub fn apply(root: &mut XastRoot, params: Option<&Value>) -> crate::error::Result<()> {
    let remove = params
        .and_then(|value| value.get("remove"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let minify = params
        .and_then(|value| value.get("minify"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let force = params
        .and_then(|value| value.get("force"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let preserve_ids = parse_stringish_set(params.and_then(|value| value.get("preserve")));
    let preserve_prefixes =
        parse_stringish_vec(params.and_then(|value| value.get("preservePrefixes")));

    let mut node_by_id = HashMap::<String, Vec<usize>>::new();
    let mut references_by_id = HashMap::<String, Vec<Reference>>::new();
    let mut reference_order = Vec::<String>::new();
    let mut deoptimized = false;

    collect_ids_and_references(
        &mut root.children,
        &mut Vec::new(),
        force,
        &mut deoptimized,
        &mut node_by_id,
        &mut references_by_id,
        &mut reference_order,
    );

    if deoptimized {
        return Ok(());
    }

    let is_id_preserved = |id: &str| {
        preserve_ids.contains(id)
            || preserve_prefixes
                .iter()
                .any(|prefix| id.starts_with(prefix.as_str()))
    };

    let mut current_id = None::<Vec<usize>>;
    for id in reference_order {
        let Some(node_path) = node_by_id.get(&id).cloned() else {
            continue;
        };

        if minify && !is_id_preserved(id.as_str()) {
            let current_id_string = loop {
                current_id = Some(generate_id(current_id.take()));
                let Some(current_id_ref) = current_id.as_ref() else {
                    continue;
                };
                let current_id_string = get_id_string(current_id_ref);
                let shadow_reference = references_by_id.contains_key(current_id_string.as_str())
                    && !node_by_id.contains_key(current_id_string.as_str());
                if !is_id_preserved(current_id_string.as_str()) && !shadow_reference {
                    break current_id_string;
                }
            };

            if let Some(node) = get_element_mut(&mut root.children, &node_path) {
                node.set_attribute("id", current_id_string.clone());
            }

            if let Some(references) = references_by_id.get(&id) {
                for reference in references {
                    if let Some(node) = get_element_mut(&mut root.children, &reference.path)
                        && let Some(value) = node.get_attribute(reference.name.as_str())
                    {
                        let next_value = if value.contains('#') {
                            value.replace(
                                format!("#{id}").as_str(),
                                format!("#{current_id_string}").as_str(),
                            )
                        } else {
                            value.replace(
                                format!("{id}.").as_str(),
                                format!("{current_id_string}.").as_str(),
                            )
                        };
                        node.set_attribute(reference.name.as_str(), next_value);
                    }
                }
            }
        }

        node_by_id.remove(&id);
    }

    if remove {
        for (id, node_path) in node_by_id {
            if !is_id_preserved(id.as_str())
                && let Some(node) = get_element_mut(&mut root.children, &node_path)
            {
                node.remove_attribute("id");
            }
        }
    }

    Ok(())
}

fn parse_stringish_set(value: Option<&Value>) -> HashSet<String> {
    parse_stringish_vec(value).into_iter().collect()
}

fn parse_stringish_vec(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect(),
        Some(Value::String(value)) => vec![value.clone()],
        _ => Vec::new(),
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_ids_and_references(
    children: &mut [XastChild],
    path: &mut Vec<usize>,
    force: bool,
    deoptimized: &mut bool,
    node_by_id: &mut HashMap<String, Vec<usize>>,
    references_by_id: &mut HashMap<String, Vec<Reference>>,
    reference_order: &mut Vec<String>,
) {
    for (index, child) in children.iter_mut().enumerate() {
        let XastChild::Element(element) = child else {
            continue;
        };
        path.push(index);

        if !force {
            if (element.name == "style" && !element.children.is_empty()) || has_scripts(element) {
                *deoptimized = true;
            }

            if element.name == "svg"
                && element
                    .children
                    .iter()
                    .all(|child| matches!(child, XastChild::Element(node) if node.name == "defs"))
            {
                path.pop();
                continue;
            }
        }

        let mut duplicate_id = false;
        for attribute in element.attributes.clone() {
            if attribute.name == "id" {
                if node_by_id.contains_key(attribute.value.as_str()) {
                    duplicate_id = true;
                } else {
                    node_by_id.insert(attribute.value, path.clone());
                }
            } else {
                for id in find_references(attribute.name.as_str(), attribute.value.as_str()) {
                    if !references_by_id.contains_key(id.as_str()) {
                        reference_order.push(id.clone());
                    }
                    references_by_id.entry(id).or_default().push(Reference {
                        path: path.clone(),
                        name: attribute.name.clone(),
                    });
                }
            }
        }
        if duplicate_id {
            element.remove_attribute("id");
        }

        collect_ids_and_references(
            &mut element.children,
            path,
            force,
            deoptimized,
            node_by_id,
            references_by_id,
            reference_order,
        );
        path.pop();
    }
}

fn generate_id(current_id: Option<Vec<usize>>) -> Vec<usize> {
    let Some(mut current_id) = current_id else {
        return vec![0];
    };
    let max_id_index = GENERATE_ID_CHARS.len() - 1;
    let last_index = current_id.len() - 1;
    current_id[last_index] += 1;
    for index in (1..current_id.len()).rev() {
        if current_id[index] > max_id_index {
            current_id[index] = 0;
            current_id[index - 1] += 1;
        }
    }
    if current_id[0] > max_id_index {
        current_id[0] = 0;
        current_id.insert(0, 0);
    }
    current_id
}

fn get_id_string(arr: &[usize]) -> String {
    arr.iter().map(|index| GENERATE_ID_CHARS[*index]).collect()
}

fn get_element_mut<'a>(children: &'a mut [XastChild], path: &[usize]) -> Option<&'a mut XastElement> {
    let (&index, rest) = path.split_first()?;
    let child = children.get_mut(index)?;
    let XastChild::Element(element) = child else {
        return None;
    };
    if rest.is_empty() {
        return Some(element);
    }
    get_element_mut(&mut element.children, rest)
}
