use std::collections::HashSet;

use serde_json::Value;

use crate::types::{XastAttribute, XastChild, XastElement, XastRoot, XastText};

const XLINK_NAMESPACE: &str = "http://www.w3.org/1999/xlink";
const LEGACY_ELEMENTS: &[&str] = &["cursor", "filter", "font-face-uri", "glyphRef", "tref"];

/// Apply the `removeXlink` plugin.
///
/// # Errors
///
/// This direct port currently does not return plugin-specific runtime errors.
pub fn apply(root: &mut XastRoot, params: Option<&Value>) -> crate::error::Result<()> {
    let include_legacy = params
        .and_then(|value| value.get("includeLegacy"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut xlink_prefixes = Vec::<String>::new();
    let mut overridden_prefixes = Vec::<String>::new();
    let mut used_in_legacy_element = HashSet::<String>::new();
    remove_xlink(
        &mut root.children,
        include_legacy,
        &mut xlink_prefixes,
        &mut overridden_prefixes,
        &mut used_in_legacy_element,
    );
    Ok(())
}

fn remove_xlink(
    children: &mut Vec<XastChild>,
    include_legacy: bool,
    xlink_prefixes: &mut Vec<String>,
    overridden_prefixes: &mut Vec<String>,
    used_in_legacy_element: &mut HashSet<String>,
) {
    for child in children {
        if let XastChild::Element(element) = child {
            process_element_enter(
                element,
                include_legacy,
                xlink_prefixes,
                overridden_prefixes,
                used_in_legacy_element,
            );
            remove_xlink(
                &mut element.children,
                include_legacy,
                xlink_prefixes,
                overridden_prefixes,
                used_in_legacy_element,
            );
            process_element_exit(
                element,
                include_legacy,
                xlink_prefixes,
                overridden_prefixes,
                used_in_legacy_element,
            );
        }
    }
}

fn process_element_enter(
    element: &mut XastElement,
    include_legacy: bool,
    xlink_prefixes: &mut Vec<String>,
    overridden_prefixes: &mut Vec<String>,
    used_in_legacy_element: &mut HashSet<String>,
) {
    for attribute in &element.attributes {
        if let Some(prefix) = attribute.name.strip_prefix("xmlns:") {
            if attribute.value == XLINK_NAMESPACE {
                xlink_prefixes.push(prefix.to_string());
                continue;
            }
            if xlink_prefixes.iter().any(|item| item == prefix) {
                overridden_prefixes.push(prefix.to_string());
            }
        }
    }

    if overridden_prefixes
        .iter()
        .any(|prefix| xlink_prefixes.iter().any(|item| item == prefix))
    {
        return;
    }

    let show_attrs = find_prefixed_attrs(element, xlink_prefixes, "show");
    let mut show_handled = element.get_attribute("target").is_some();
    for attr in show_attrs.iter().rev() {
        let Some(value) = element.get_attribute(attr).map(str::to_string) else {
            continue;
        };
        let mapping = match value.as_str() {
            "new" => Some("_blank"),
            "replace" => Some("_self"),
            _ => None,
        };
        if show_handled || mapping.is_none() {
            element.remove_attribute(attr);
            continue;
        }
        let mapping = mapping.expect("mapping checked");
        if Some(mapping) != default_target(element.name.as_str()) {
            element.set_attribute("target", mapping.to_string());
        }
        element.remove_attribute(attr);
        show_handled = true;
    }

    let title_attrs = find_prefixed_attrs(element, xlink_prefixes, "title");
    for attr in title_attrs.iter().rev() {
        let Some(value) = element.get_attribute(attr).map(str::to_string) else {
            continue;
        };
        if has_title_child(element) {
            element.remove_attribute(attr);
            continue;
        }
        element.children.insert(
            0,
            XastChild::Element(XastElement {
                name: "title".to_string(),
                attributes: Vec::new(),
                children: vec![XastChild::Text(XastText { value })],
            }),
        );
        element.remove_attribute(attr);
    }

    let href_attrs = find_prefixed_attrs(element, xlink_prefixes, "href");
    if !href_attrs.is_empty() && LEGACY_ELEMENTS.contains(&element.name.as_str()) && !include_legacy
    {
        for attr in href_attrs {
            if let Some((prefix, _)) = attr.split_once(':') {
                used_in_legacy_element.insert(prefix.to_string());
            }
        }
        return;
    }

    for attr in href_attrs.iter().rev() {
        let Some(value) = element.get_attribute(attr).map(str::to_string) else {
            continue;
        };
        if element.get_attribute("href").is_some() {
            element.remove_attribute(attr);
            continue;
        }
        element.set_attribute("href", value);
        element.remove_attribute(attr);
    }
}

fn process_element_exit(
    element: &mut XastElement,
    include_legacy: bool,
    xlink_prefixes: &mut Vec<String>,
    overridden_prefixes: &mut Vec<String>,
    used_in_legacy_element: &HashSet<String>,
) {
    let mut index = 0usize;
    while index < element.attributes.len() {
        let key = element.attributes[index].name.clone();
        let value = element.attributes[index].value.clone();
        let (prefix, attr) = key.split_once(':').unwrap_or(("", ""));

        if !include_legacy
            && xlink_prefixes.iter().any(|item| item == prefix)
            && !overridden_prefixes.iter().any(|item| item == prefix)
            && !used_in_legacy_element.contains(prefix)
        {
            element.attributes.remove(index);
            continue;
        }

        if key.starts_with("xmlns:") && !used_in_legacy_element.contains(attr) {
            if value == XLINK_NAMESPACE {
                remove_first_matching(xlink_prefixes, attr);
                element.attributes.remove(index);
                continue;
            }

            if overridden_prefixes.iter().any(|item| item == attr) {
                remove_first_matching(overridden_prefixes, attr);
            }
        }

        index += 1;
    }
}

fn find_prefixed_attrs(element: &XastElement, prefixes: &[String], attr: &str) -> Vec<String> {
    prefixes
        .iter()
        .map(|prefix| format!("{prefix}:{attr}"))
        .filter(|name| element.get_attribute(name).is_some())
        .collect()
}

fn remove_first_matching(items: &mut Vec<String>, needle: &str) {
    if let Some(index) = items.iter().position(|item| item == needle) {
        items.remove(index);
    }
}

fn has_title_child(element: &XastElement) -> bool {
    element
        .children
        .iter()
        .any(|child| matches!(child, XastChild::Element(node) if node.name == "title"))
}

fn default_target(name: &str) -> Option<&'static str> {
    (name == "a").then_some("_self")
}

#[allow(dead_code)]
fn _attribute(name: &str, value: String) -> XastAttribute {
    XastAttribute {
        name: name.to_string(),
        value,
    }
}
