use serde_json::Value;

use crate::error::Result;
use crate::types::{XastChild, XastRoot};

/// Apply the `removeElementsByAttr` plugin.
///
/// # Errors
///
/// This direct port currently does not return plugin-specific runtime errors.
pub fn apply(root: &mut XastRoot, params: Option<&Value>) -> Result<()> {
    let Some(params) = params else {
        return Ok(());
    };

    let ids = read_string_list(params.get("id"));
    let classes = read_string_list(params.get("class"));
    remove_matching_elements(&mut root.children, &ids, &classes);
    Ok(())
}

fn read_string_list(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::String(item)) => vec![item.clone()],
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

fn remove_matching_elements(children: &mut Vec<XastChild>, ids: &[String], classes: &[String]) {
    let mut index = 0usize;
    while index < children.len() {
        let mut removed = false;
        if let XastChild::Element(element) = &mut children[index] {
            let id_matches = !ids.is_empty()
                && element
                    .get_attribute("id")
                    .is_some_and(|value| ids.iter().any(|id| id == value));
            let class_matches = !classes.is_empty()
                && element.get_attribute("class").is_some_and(|value| {
                    let class_list = value.split_ascii_whitespace().collect::<Vec<_>>();
                    classes
                        .iter()
                        .any(|class_name| class_list.iter().any(|class| class == class_name))
                });

            if id_matches || class_matches {
                children.remove(index);
                removed = true;
            } else {
                remove_matching_elements(&mut element.children, ids, classes);
            }
        }

        if !removed {
            index += 1;
        }
    }
}
