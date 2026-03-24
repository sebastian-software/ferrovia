use regex::Regex;
use serde_json::Value;

use crate::types::{XastChild, XastRoot};

/// Apply the `cleanupAttrs` plugin.
///
/// # Errors
///
/// This direct port currently does not return plugin-specific runtime errors.
///
/// # Panics
///
/// Panics if one of the hard-coded cleanup regexes becomes invalid.
pub fn apply(root: &mut XastRoot, params: Option<&Value>) -> crate::error::Result<()> {
    let newlines = params
        .and_then(|value| value.get("newlines"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let trim = params
        .and_then(|value| value.get("trim"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let spaces = params
        .and_then(|value| value.get("spaces"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let reg_newlines_need_space = Regex::new(r"(\S)\r?\n(\S)").expect("valid newline-space regex");
    let reg_newlines = Regex::new(r"\r?\n").expect("valid newline regex");
    let reg_spaces = Regex::new(r"\s{2,}").expect("valid space regex");
    cleanup_attrs(
        &mut root.children,
        newlines,
        trim,
        spaces,
        &reg_newlines_need_space,
        &reg_newlines,
        &reg_spaces,
    );
    Ok(())
}

fn cleanup_attrs(
    children: &mut [XastChild],
    newlines: bool,
    trim: bool,
    spaces: bool,
    reg_newlines_need_space: &Regex,
    reg_newlines: &Regex,
    reg_spaces: &Regex,
) {
    for child in children {
        if let XastChild::Element(element) = child {
            for attribute in &mut element.attributes {
                if newlines {
                    attribute.value = reg_newlines_need_space
                        .replace_all(attribute.value.as_str(), "$1 $2")
                        .into_owned();
                    attribute.value = reg_newlines
                        .replace_all(attribute.value.as_str(), "")
                        .into_owned();
                }
                if trim {
                    attribute.value = attribute.value.trim().to_string();
                }
                if spaces {
                    attribute.value = reg_spaces
                        .replace_all(attribute.value.as_str(), " ")
                        .into_owned();
                }
            }
            cleanup_attrs(
                &mut element.children,
                newlines,
                trim,
                spaces,
                reg_newlines_need_space,
                reg_newlines,
                reg_spaces,
            );
        }
    }
}
