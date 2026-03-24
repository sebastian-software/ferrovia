use serde_json::Value;

use crate::types::{XastChild, XastRoot};

const STANDARD_DESCS: [&str; 2] = ["Created with", "Created using"];

/// Apply the `removeDesc` plugin.
///
/// # Errors
///
/// This direct port currently does not return plugin-specific runtime errors.
pub fn apply(root: &mut XastRoot, params: Option<&Value>) -> crate::error::Result<()> {
    let remove_any = params
        .and_then(|value| value.get("removeAny"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    remove_desc(&mut root.children, remove_any);
    Ok(())
}

fn remove_desc(children: &mut Vec<XastChild>, remove_any: bool) {
    let mut index = 0usize;
    while index < children.len() {
        let mut removed = false;
        match &mut children[index] {
            XastChild::Element(element) if element.name == "desc" => {
                let should_remove = remove_any
                    || element.children.is_empty()
                    || matches!(
                        element.children.first(),
                        Some(XastChild::Text(text))
                            if STANDARD_DESCS
                                .iter()
                                .any(|prefix| text.value.starts_with(prefix))
                    );
                if should_remove {
                    children.remove(index);
                    removed = true;
                }
            }
            XastChild::Element(element) => remove_desc(&mut element.children, remove_any),
            _ => {}
        }
        if !removed {
            index += 1;
        }
    }
}
