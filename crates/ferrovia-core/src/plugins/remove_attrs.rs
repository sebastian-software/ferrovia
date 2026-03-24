use regex::Regex;
use serde_json::Value;

use crate::error::{FerroviaError, Result};
use crate::types::{XastChild, XastRoot};

const DEFAULT_SEPARATOR: &str = ":";

/// Apply the `removeAttrs` plugin.
///
/// # Errors
///
/// Returns an error if one of the configured attribute patterns cannot be compiled as a regular expression.
pub fn apply(root: &mut XastRoot, params: Option<&Value>) -> Result<()> {
    let Some(params) = params else {
        return Ok(());
    };
    let Some(raw_attrs) = params.get("attrs") else {
        return Ok(());
    };

    let elem_separator = params
        .get("elemSeparator")
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_SEPARATOR);
    let preserve_current_color = params
        .get("preserveCurrentColor")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let attrs = raw_attrs.as_array().map_or_else(
        || {
            raw_attrs
                .as_str()
                .map_or_else(Vec::new, |single| vec![single])
        },
        |array| array.iter().filter_map(Value::as_str).collect::<Vec<_>>(),
    );

    let patterns = attrs
        .into_iter()
        .map(|pattern| compile_pattern(pattern, elem_separator))
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|error| FerroviaError::InvalidConfig(error.to_string()))?;

    remove_matching_attributes(&mut root.children, &patterns, preserve_current_color);
    Ok(())
}

struct CompiledPattern {
    element: Regex,
    attribute: Regex,
    value: Regex,
}

fn compile_pattern(
    pattern: &str,
    elem_separator: &str,
) -> std::result::Result<CompiledPattern, regex::Error> {
    let normalized = if !pattern.contains(elem_separator) {
        [" .*".trim(), pattern, ".*"].join(elem_separator)
    } else if pattern.split(elem_separator).count() < 3 {
        [pattern, ".*"].join(elem_separator)
    } else {
        pattern.to_string()
    };

    let mut parts = normalized
        .split(elem_separator)
        .map(|value| if value == "*" { ".*" } else { value })
        .collect::<Vec<_>>();
    while parts.len() < 3 {
        parts.push(".*");
    }

    Ok(CompiledPattern {
        element: Regex::new(format!("^{}$", parts[0]).as_str())?,
        attribute: Regex::new(format!("^{}$", parts[1]).as_str())?,
        value: Regex::new(format!("^{}$", parts[2]).as_str())?,
    })
}

fn remove_matching_attributes(
    children: &mut [XastChild],
    patterns: &[CompiledPattern],
    preserve_current_color: bool,
) {
    for child in children {
        if let XastChild::Element(element) = child {
            for pattern in patterns {
                if !pattern.element.is_match(element.name.as_str()) {
                    continue;
                }

                element.attributes.retain(|attribute| {
                    let is_current_color = attribute.value.eq_ignore_ascii_case("currentcolor");
                    let is_fill_current_color =
                        preserve_current_color && attribute.name == "fill" && is_current_color;
                    let is_stroke_current_color =
                        preserve_current_color && attribute.name == "stroke" && is_current_color;

                    is_fill_current_color
                        || is_stroke_current_color
                        || !pattern.attribute.is_match(attribute.name.as_str())
                        || !pattern.value.is_match(attribute.value.as_str())
                });
            }

            remove_matching_attributes(&mut element.children, patterns, preserve_current_color);
        }
    }
}
