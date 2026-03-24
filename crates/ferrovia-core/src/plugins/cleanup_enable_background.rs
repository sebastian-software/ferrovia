use regex::Regex;

use crate::style::parse_style_declarations;
use crate::types::{StylesheetDeclaration, XastChild, XastElement, XastRoot};

/// Apply the `cleanupEnableBackground` plugin.
///
/// # Errors
///
/// This direct port currently does not return plugin-specific runtime errors.
///
/// # Panics
///
/// Panics if the hard-coded `enable-background` regex becomes invalid.
pub fn apply(root: &mut XastRoot) -> crate::error::Result<()> {
    let has_filter = has_filter(&root.children);
    let regex = Regex::new(
        r"^new\s0\s0\s([-+]?\d*\.?\d+(?:[eE][-+]?\d+)?)\s([-+]?\d*\.?\d+(?:[eE][-+]?\d+)?)$",
    )
    .expect("valid enable-background regex");
    cleanup_enable_background(&mut root.children, has_filter, &regex);
    Ok(())
}

fn has_filter(children: &[XastChild]) -> bool {
    for child in children {
        if let XastChild::Element(element) = child
            && (element.name == "filter" || has_filter(&element.children))
        {
            return true;
        }
    }
    false
}

fn cleanup_enable_background(children: &mut [XastChild], has_filter: bool, regex: &Regex) {
    for child in children {
        if let XastChild::Element(element) = child {
            let mut style = element
                .get_attribute("style")
                .map(parse_style_declarations)
                .unwrap_or_default();
            let last_enable_background = style
                .iter()
                .enumerate()
                .filter(|(_, declaration)| declaration.name == "enable-background")
                .map(|(index, _)| index)
                .next_back();

            if let Some(last_index) = last_enable_background {
                style = style
                    .into_iter()
                    .enumerate()
                    .filter_map(|(index, declaration)| {
                        (declaration.name != "enable-background" || index == last_index)
                            .then_some(declaration)
                    })
                    .collect();
            }

            if !has_filter {
                element.remove_attribute("enable-background");
                if let Some(index) = style
                    .iter()
                    .position(|declaration| declaration.name == "enable-background")
                {
                    style.remove(index);
                }
                write_style_attribute(element, style);
                cleanup_enable_background(&mut element.children, has_filter, regex);
                continue;
            }

            let has_dimensions =
                element.get_attribute("width").is_some() && element.get_attribute("height").is_some();
            if (element.name == "svg" || element.name == "mask" || element.name == "pattern")
                && has_dimensions
            {
                let width = element.get_attribute("width").map(str::to_string);
                let height = element.get_attribute("height").map(str::to_string);
                if let (Some(width), Some(height)) = (width, height) {
                    if let Some(value) = element.get_attribute("enable-background") {
                        match cleanup_value(
                            value,
                            element.name.as_str(),
                            width.as_str(),
                            height.as_str(),
                            regex,
                        ) {
                            Some(cleaned) => element.set_attribute("enable-background", cleaned),
                            None => {
                                element.remove_attribute("enable-background");
                            }
                        }
                    }

                    if let Some(index) = style
                        .iter()
                        .position(|declaration| declaration.name == "enable-background")
                    {
                        let value = style[index].value.clone();
                        match cleanup_value(
                            value.as_str(),
                            element.name.as_str(),
                            width.as_str(),
                            height.as_str(),
                            regex,
                        ) {
                            Some(cleaned) => {
                                style[index].value = cleaned;
                            }
                            None => {
                                style.remove(index);
                            }
                        }
                    }
                }
            }

            write_style_attribute(element, style);
            cleanup_enable_background(&mut element.children, has_filter, regex);
        }
    }
}

fn cleanup_value(
    value: &str,
    node_name: &str,
    width: &str,
    height: &str,
    regex: &Regex,
) -> Option<String> {
    let captures = regex.captures(value)?;
    let width_match = captures.get(1)?.as_str();
    let height_match = captures.get(2)?.as_str();
    if width == width_match && height == height_match {
        return (node_name != "svg").then_some("new".to_string());
    }
    Some(value.to_string())
}

fn write_style_attribute(element: &mut XastElement, declarations: Vec<StylesheetDeclaration>) {
    if declarations.is_empty() {
        element.remove_attribute("style");
        return;
    }
    let style = declarations
        .into_iter()
        .map(|declaration| format!("{}:{}", declaration.name, declaration.value))
        .collect::<Vec<_>>()
        .join(";");
    element.set_attribute("style", style);
}
