use std::collections::HashSet;

use ferrovia_css_what_compat::parse as parse_selector_groups;
use serde_json::Value;

use crate::plugins::_collections::{
    DeprecatedAttrs, deprecated_attrs_group, deprecated_elem_config,
};
use crate::style::collect_stylesheet;
use crate::types::{Stylesheet, XastChild, XastElement, XastRoot};

/// Apply the `removeDeprecatedAttrs` plugin.
///
/// # Errors
///
/// This direct port currently does not return plugin-specific runtime errors.
pub fn apply(root: &mut XastRoot, params: Option<&Value>) -> crate::error::Result<()> {
    let remove_unsafe = params
        .and_then(|value| value.get("removeUnsafe"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let stylesheet = collect_stylesheet(root);
    let attributes_in_stylesheet = extract_attributes_in_stylesheet(&stylesheet);
    remove_deprecated_attrs(&mut root.children, remove_unsafe, &attributes_in_stylesheet);
    Ok(())
}

fn extract_attributes_in_stylesheet(stylesheet: &Stylesheet) -> HashSet<String> {
    let mut attributes_in_stylesheet = HashSet::<String>::new();
    for rule in &stylesheet.rules {
        for selector_group in parse_selector_groups(rule.selector.as_str()) {
            for token in selector_group.tokens {
                for attribute in token.compound.attributes {
                    attributes_in_stylesheet.insert(attribute.name);
                }
            }
        }
    }
    attributes_in_stylesheet
}

fn remove_deprecated_attrs(
    children: &mut [XastChild],
    remove_unsafe: bool,
    attributes_in_stylesheet: &HashSet<String>,
) {
    for child in children {
        if let XastChild::Element(element) = child {
            process_element(element, remove_unsafe, attributes_in_stylesheet);
            remove_deprecated_attrs(
                &mut element.children,
                remove_unsafe,
                attributes_in_stylesheet,
            );
        }
    }
}

fn process_element(
    element: &mut XastElement,
    remove_unsafe: bool,
    attributes_in_stylesheet: &HashSet<String>,
) {
    let Some(config) = deprecated_elem_config(element.name.as_str()) else {
        return;
    };

    if config.attrs_groups.contains(&"core")
        && element.get_attribute("xml:lang").is_some()
        && !attributes_in_stylesheet.contains("xml:lang")
        && element.get_attribute("lang").is_some()
    {
        element.remove_attribute("xml:lang");
    }

    for attrs_group in config.attrs_groups {
        if let Some(deprecated_attrs) = deprecated_attrs_group(attrs_group) {
            process_attributes(
                element,
                deprecated_attrs,
                remove_unsafe,
                attributes_in_stylesheet,
            );
        }
    }

    if let Some(deprecated_attrs) = config.deprecated {
        process_attributes(
            element,
            deprecated_attrs,
            remove_unsafe,
            attributes_in_stylesheet,
        );
    }
}

fn process_attributes(
    element: &mut XastElement,
    deprecated_attrs: DeprecatedAttrs,
    remove_unsafe: bool,
    attributes_in_stylesheet: &HashSet<String>,
) {
    for name in deprecated_attrs.safe {
        if !attributes_in_stylesheet.contains(*name) {
            element.remove_attribute(name);
        }
    }

    if remove_unsafe {
        for name in deprecated_attrs.unsafe_attrs {
            if !attributes_in_stylesheet.contains(*name) {
                element.remove_attribute(name);
            }
        }
    }
}
