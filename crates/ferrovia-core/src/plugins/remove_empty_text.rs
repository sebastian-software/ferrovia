use serde_json::Value;

use crate::types::{XastChild, XastRoot};

/// Apply the `removeEmptyText` plugin.
///
/// # Errors
///
/// This direct port currently does not return plugin-specific runtime errors.
pub fn apply(root: &mut XastRoot, params: Option<&Value>) -> crate::error::Result<()> {
    let text = params
        .and_then(|value| value.get("text"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let tspan = params
        .and_then(|value| value.get("tspan"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let tref = params
        .and_then(|value| value.get("tref"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    remove_empty_text(&mut root.children, text, tspan, tref);
    Ok(())
}

fn remove_empty_text(children: &mut Vec<XastChild>, text: bool, tspan: bool, tref: bool) {
    let mut index = 0usize;
    while index < children.len() {
        let mut removed = false;
        match &mut children[index] {
            XastChild::Element(element)
                if (text && element.name == "text" && element.children.is_empty())
                    || (tspan && element.name == "tspan" && element.children.is_empty())
                    || (tref
                        && element.name == "tref"
                        && element.get_attribute("xlink:href").is_none()) =>
            {
                children.remove(index);
                removed = true;
            }
            XastChild::Element(element) => {
                remove_empty_text(&mut element.children, text, tspan, tref);
            }
            _ => {}
        }

        if !removed {
            index += 1;
        }
    }
}
