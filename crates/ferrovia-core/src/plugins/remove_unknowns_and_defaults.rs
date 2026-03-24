use std::collections::{HashMap, HashSet};

use regex::Regex;
use serde_json::Value;

use crate::error::Result;
use crate::plugins::_collections::is_presentation_non_inheritable_group_attr;
use crate::style::{collect_stylesheet, compute_style, includes_attr_selector};
use crate::types::{ComputedStyle, XastChild, XastElement, XastInstruction, XastRoot};

/// Direct port slice of SVGO's `removeUnknownsAndDefaults`.
///
/// # Errors
///
/// This direct port currently performs in-memory tree rewrites only and does
/// not produce operational errors.
pub fn apply(root: &mut XastRoot, params: Option<&Value>) -> Result<()> {
    let unknown_content = bool_param(params, "unknownContent", true);
    let unknown_attrs = bool_param(params, "unknownAttrs", true);
    let default_attrs = bool_param(params, "defaultAttrs", true);
    let default_markup_declarations = bool_param(params, "defaultMarkupDeclarations", true);
    let useless_overrides = bool_param(params, "uselessOverrides", true);
    let keep_data_attrs = bool_param(params, "keepDataAttrs", true);
    let keep_aria_attrs = bool_param(params, "keepAriaAttrs", true);
    let keep_role_attr = bool_param(params, "keepRoleAttr", false);

    if default_markup_declarations {
        remove_default_markup_declarations(&mut root.children);
    }

    let stylesheet = collect_stylesheet(root);
    let collections = Collections::default();
    rewrite_children(
        &mut root.children,
        None,
        &stylesheet,
        &collections,
        RewriteOptions {
            unknown_content,
            unknown_attrs,
            default_attrs,
            useless_overrides,
            keep_data_attrs,
            keep_aria_attrs,
            keep_role_attr,
        },
    );
    Ok(())
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy)]
struct RewriteOptions {
    unknown_content: bool,
    unknown_attrs: bool,
    default_attrs: bool,
    useless_overrides: bool,
    keep_data_attrs: bool,
    keep_aria_attrs: bool,
    keep_role_attr: bool,
}

#[allow(clippy::struct_field_names)]
#[derive(Debug, Clone)]
struct Collections {
    allowed_children_per_element: HashMap<&'static str, HashSet<&'static str>>,
    allowed_attributes_per_element: HashMap<&'static str, HashSet<&'static str>>,
    attributes_defaults_per_element: HashMap<&'static str, HashMap<&'static str, &'static str>>,
}

impl Default for Collections {
    fn default() -> Self {
        let mut allowed_children_per_element = HashMap::new();
        allowed_children_per_element.insert("svg", set(&[
            "a", "animate", "animateMotion", "animateTransform", "circle", "clipPath", "defs",
            "desc", "ellipse", "filter", "foreignObject", "g", "image", "line", "linearGradient",
            "mask", "metadata", "path", "pattern", "polygon", "polyline", "radialGradient",
            "rect", "script", "set", "style", "switch", "symbol", "text", "title", "use",
        ]));
        allowed_children_per_element.insert("g", allowed_children_per_element["svg"].clone());
        allowed_children_per_element.insert("defs", set(&[
            "a", "animate", "animateTransform", "circle", "clipPath", "desc", "ellipse", "filter",
            "g", "image", "line", "linearGradient", "mask", "metadata", "path", "pattern",
            "polygon", "polyline", "radialGradient", "rect", "script", "set", "style", "symbol",
            "text", "title", "use",
        ]));
        allowed_children_per_element.insert("linearGradient", set(&["animate", "animateTransform", "set", "stop", "desc", "metadata", "title"]));
        allowed_children_per_element.insert("radialGradient", allowed_children_per_element["linearGradient"].clone());
        allowed_children_per_element.insert("text", set(&["a", "animate", "animateTransform", "set", "textPath", "tspan", "tref"]));
        allowed_children_per_element.insert("a", allowed_children_per_element["svg"].clone());
        allowed_children_per_element.insert("symbol", allowed_children_per_element["svg"].clone());
        allowed_children_per_element.insert("switch", allowed_children_per_element["svg"].clone());

        let common_attrs = set(&["id", "class", "style", "transform", "xml:lang", "xml:space"]);
        let presentation_attrs = set(&[
            "fill", "fill-opacity", "fill-rule", "stroke", "stroke-width", "stroke-opacity",
            "stroke-linecap", "stroke-linejoin", "stroke-miterlimit", "stroke-dasharray",
            "stroke-dashoffset", "opacity", "display", "visibility", "clip-path", "mask",
            "filter", "marker-start", "marker-mid", "marker-end", "stop-color", "stop-opacity",
            "text-anchor", "font-size", "font-family",
        ]);

        let mut allowed_attributes_per_element = HashMap::new();
        allowed_attributes_per_element.insert("svg", union(&common_attrs, &set(&["xmlns", "width", "height", "viewBox", "x", "y"])));
        allowed_attributes_per_element.insert("g", union_many(&[&common_attrs, &presentation_attrs]));
        allowed_attributes_per_element.insert("path", union_many(&[&common_attrs, &presentation_attrs, &set(&["d", "pathLength"])]));
        allowed_attributes_per_element.insert("rect", union_many(&[&common_attrs, &presentation_attrs, &set(&["x", "y", "width", "height", "rx", "ry"])]));
        allowed_attributes_per_element.insert("circle", union_many(&[&common_attrs, &presentation_attrs, &set(&["cx", "cy", "r"])]));
        allowed_attributes_per_element.insert("ellipse", union_many(&[&common_attrs, &presentation_attrs, &set(&["cx", "cy", "rx", "ry"])]));
        allowed_attributes_per_element.insert("line", union_many(&[&common_attrs, &presentation_attrs, &set(&["x1", "y1", "x2", "y2"])]));
        allowed_attributes_per_element.insert("polyline", union_many(&[&common_attrs, &presentation_attrs, &set(&["points"])]));
        allowed_attributes_per_element.insert("polygon", union_many(&[&common_attrs, &presentation_attrs, &set(&["points"])]));
        allowed_attributes_per_element.insert("text", union_many(&[&common_attrs, &presentation_attrs, &set(&["x", "y", "dx", "dy", "rotate", "textLength", "lengthAdjust"])]));
        allowed_attributes_per_element.insert("use", union_many(&[&common_attrs, &presentation_attrs, &set(&["x", "y", "width", "height", "href", "xlink:href"])]));
        allowed_attributes_per_element.insert("image", union_many(&[&common_attrs, &presentation_attrs, &set(&["x", "y", "width", "height", "href", "xlink:href"])]));
        allowed_attributes_per_element.insert("style", set(&["type", "media", "title"]));
        allowed_attributes_per_element.insert("linearGradient", union_many(&[&common_attrs, &set(&["x1", "y1", "x2", "y2", "gradientUnits", "gradientTransform", "spreadMethod", "href", "xlink:href"])]));
        allowed_attributes_per_element.insert("radialGradient", union_many(&[&common_attrs, &set(&["cx", "cy", "r", "fx", "fy", "fr", "gradientUnits", "gradientTransform", "spreadMethod", "href", "xlink:href"])]));
        allowed_attributes_per_element.insert("stop", union_many(&[&common_attrs, &presentation_attrs, &set(&["offset"])]));
        allowed_attributes_per_element.insert("foreignObject", union_many(&[&common_attrs, &presentation_attrs, &set(&["x", "y", "width", "height"])]));
        allowed_attributes_per_element.insert("defs", common_attrs.clone());
        allowed_attributes_per_element.insert("desc", common_attrs.clone());
        allowed_attributes_per_element.insert("title", common_attrs.clone());
        allowed_attributes_per_element.insert("metadata", common_attrs.clone());
        allowed_attributes_per_element.insert("animate", union_many(&[&common_attrs, &set(&["attributeName", "attributeType", "from", "to", "values", "dur", "begin", "end", "repeatCount", "repeatDur", "calcMode", "keyTimes", "keySplines", "fill", "additive", "accumulate"])]));
        allowed_attributes_per_element.insert("animateTransform", union_many(&[&common_attrs, &set(&["attributeName", "attributeType", "type", "from", "to", "values", "dur", "begin", "end", "repeatCount", "repeatDur", "calcMode", "keyTimes", "keySplines", "fill", "additive", "accumulate"])]));
        allowed_attributes_per_element.insert("set", union_many(&[&common_attrs, &set(&["attributeName", "attributeType", "to", "begin", "end", "dur", "fill"])]));

        let mut attributes_defaults_per_element = HashMap::new();
        let presentation_defaults = HashMap::from([
            ("clip-path", "none"),
            ("display", "inline"),
            ("fill", "#000"),
            ("fill-opacity", "1"),
            ("fill-rule", "nonzero"),
            ("mask", "none"),
            ("marker-start", "none"),
            ("marker-mid", "none"),
            ("marker-end", "none"),
            ("opacity", "1"),
            ("stop-color", "#000"),
            ("stop-opacity", "1"),
            ("stroke", "none"),
            ("stroke-width", "1"),
            ("stroke-linecap", "butt"),
            ("stroke-linejoin", "miter"),
            ("stroke-miterlimit", "4"),
            ("stroke-dasharray", "none"),
            ("stroke-dashoffset", "0"),
            ("stroke-opacity", "1"),
            ("text-anchor", "start"),
            ("visibility", "visible"),
            ("xml:space", "default"),
        ]);
        for name in [
            "g", "path", "rect", "circle", "ellipse", "line", "polyline", "polygon", "text",
            "use", "image", "stop", "svg",
        ] {
            attributes_defaults_per_element.insert(name, presentation_defaults.clone());
        }
        attributes_defaults_per_element.insert("svg", HashMap::from([("x", "0"), ("y", "0")]));
        attributes_defaults_per_element.insert("use", HashMap::from([("x", "0"), ("y", "0")]));
        attributes_defaults_per_element.insert("image", HashMap::from([("x", "0"), ("y", "0")]));
        attributes_defaults_per_element.insert("linearGradient", HashMap::from([("x1", "0%"), ("y1", "0%"), ("x2", "100%"), ("y2", "0%")]));
        attributes_defaults_per_element.insert("radialGradient", HashMap::from([("cx", "50%"), ("cy", "50%"), ("r", "50%"), ("fx", "50%"), ("fy", "50%")]));

        Self {
            allowed_children_per_element,
            allowed_attributes_per_element,
            attributes_defaults_per_element,
        }
    }
}

fn rewrite_children(
    children: &mut Vec<XastChild>,
    parent: Option<&XastElement>,
    stylesheet: &crate::types::Stylesheet,
    collections: &Collections,
    options: RewriteOptions,
) {
    let mut index = 0usize;
    while index < children.len() {
        let mut remove_current = false;

        if let XastChild::Element(node) = &mut children[index] {
            if options.unknown_content && parent.is_some_and(|parent| should_remove_child(parent, node, collections)) {
                remove_current = true;
            } else if node.name.contains(':') {
                let parent_snapshot = node.clone();
                rewrite_children(
                    &mut node.children,
                    Some(&parent_snapshot),
                    stylesheet,
                    collections,
                    options,
                );
            } else if node.name != "foreignObject" {
                let parent_style = parent.map(|element| compute_style(stylesheet, element));
                cleanup_attributes(node, parent_style.as_ref(), stylesheet, collections, options);
                let parent_snapshot = node.clone();
                rewrite_children(
                    &mut node.children,
                    Some(&parent_snapshot),
                    stylesheet,
                    collections,
                    options,
                );
            }
        }

        if remove_current {
            children.remove(index);
        } else {
            index += 1;
        }
    }
}

fn cleanup_attributes(
    node: &mut XastElement,
    parent_style: Option<&Vec<(String, ComputedStyle)>>,
    stylesheet: &crate::types::Stylesheet,
    collections: &Collections,
    options: RewriteOptions,
) {
    let allowed_attributes = collections.allowed_attributes_per_element.get(node.name.as_str());
    let attributes_defaults = collections
        .attributes_defaults_per_element
        .get(node.name.as_str());

    let mut index = 0usize;
    while index < node.attributes.len() {
        let name = node.attributes[index].name.clone();
        let value = node.attributes[index].value.clone();

        if options.keep_data_attrs && name.starts_with("data-") {
            index += 1;
            continue;
        }
        if options.keep_aria_attrs && name.starts_with("aria-") {
            index += 1;
            continue;
        }
        if options.keep_role_attr && name == "role" {
            index += 1;
            continue;
        }
        if name == "xmlns" {
            index += 1;
            continue;
        }
        if name.contains(':') {
            let prefix = name.split(':').next().unwrap_or_default();
            if prefix != "xml" && prefix != "xlink" {
                index += 1;
                continue;
            }
        }

        let mut remove = if options.unknown_attrs {
            allowed_attributes.is_some_and(|allowed_attributes| {
                !allowed_attributes.contains(name.as_str())
            })
        } else {
            false
        };

        if !remove
            && options.default_attrs
            && node.get_attribute("id").is_none()
            && let Some(defaults) = attributes_defaults
            && defaults.get(name.as_str()).is_some_and(|default| *default == value)
            && parent_style
                .is_none_or(|style| !style.iter().any(|(style_name, _)| style_name == &name))
            && !stylesheet
                .rules
                .iter()
                .any(|rule| includes_attr_selector(rule.selector.as_str(), name.as_str(), None, false))
        {
            remove = true;
        }

        if !remove
            && options.useless_overrides
            && node.get_attribute("id").is_none()
            && !is_presentation_non_inheritable_group_attr(name.as_str())
            && let Some(parent_style) = parent_style
            && let Some((_, ComputedStyle::Static { value: parent_value, .. })) = parent_style
                .iter()
                .find(|(style_name, _)| style_name == &name)
            && parent_value == &value
        {
            remove = true;
        }

        if remove {
            node.attributes.remove(index);
        } else {
            index += 1;
        }
    }
}

fn should_remove_child(parent: &XastElement, child: &XastElement, collections: &Collections) -> bool {
    let Some(allowed_children) = collections.allowed_children_per_element.get(parent.name.as_str()) else {
        return !collections.allowed_children_per_element.contains_key(child.name.as_str());
    };
    !allowed_children.contains(child.name.as_str())
}

fn remove_default_markup_declarations(children: &mut [XastChild]) {
    let Ok(double_quoted) = Regex::new(r#"\s*standalone\s*=\s*"no""#) else {
        return;
    };
    let Ok(single_quoted) = Regex::new(r"\s*standalone\s*=\s*'no'") else {
        return;
    };
    for child in children {
        if let XastChild::Instruction(XastInstruction { name, value }) = child
            && name == "xml"
        {
            *value = double_quoted.replace(value, "").to_string();
            *value = single_quoted.replace(value, "").to_string();
        }
    }
}

fn bool_param(params: Option<&Value>, name: &str, default: bool) -> bool {
    params
        .and_then(|value| value.get(name))
        .and_then(Value::as_bool)
        .unwrap_or(default)
}

fn set(values: &[&'static str]) -> HashSet<&'static str> {
    values.iter().copied().collect()
}

fn union(left: &HashSet<&'static str>, right: &HashSet<&'static str>) -> HashSet<&'static str> {
    left.union(right).copied().collect()
}

fn union_many(sets: &[&HashSet<&'static str>]) -> HashSet<&'static str> {
    let mut result = HashSet::new();
    for set in sets {
        result.extend(set.iter().copied());
    }
    result
}
