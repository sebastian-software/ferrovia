use std::collections::HashSet;

use serde_json::Value;

use crate::path::parse_path_data;
use crate::plugins::_collections::is_non_rendering_elem;
use crate::style::{collect_stylesheet, compute_style};
use crate::svgo::tools::{find_references, has_scripts};
use crate::types::{ComputedStyle, Stylesheet, XastChild, XastElement, XastRoot};

/// Apply the `removeHiddenElems` plugin.
///
/// # Errors
///
/// This direct port currently does not return plugin-specific runtime errors.
pub fn apply(root: &mut XastRoot, params: Option<&Value>) -> crate::error::Result<()> {
    let options = RemoveHiddenElemsOptions::from_params(params);
    let stylesheet = collect_stylesheet(root);
    let global_state = collect_global_state(root);

    let mut removed_def_ids = HashSet::<String>::new();
    remove_hidden_elems(
        &mut root.children,
        None,
        &stylesheet,
        &global_state.all_references,
        global_state.deoptimized,
        options,
        &mut removed_def_ids,
    );

    if !removed_def_ids.is_empty() {
        remove_use_nodes_referencing_ids(&mut root.children, &removed_def_ids);
        if !global_state.deoptimized {
            let all_references = collect_all_references(root);
            remove_non_rendering_with_updated_references(
                &mut root.children,
                None,
                &stylesheet,
                &all_references,
                options,
            );
        }
    }

    remove_empty_defs(&mut root.children);
    Ok(())
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy)]
struct RemoveHiddenElemsOptions {
    is_hidden: bool,
    display_none: bool,
    opacity0: bool,
    circle_r0: bool,
    ellipse_rx0: bool,
    ellipse_ry0: bool,
    rect_width0: bool,
    rect_height0: bool,
    pattern_width0: bool,
    pattern_height0: bool,
    image_width0: bool,
    image_height0: bool,
    path_empty_d: bool,
    polyline_empty_points: bool,
    polygon_empty_points: bool,
}

impl RemoveHiddenElemsOptions {
    fn from_params(params: Option<&Value>) -> Self {
        Self {
            is_hidden: bool_param(params, "isHidden", true),
            display_none: bool_param(params, "displayNone", true),
            opacity0: bool_param(params, "opacity0", true),
            circle_r0: bool_param(params, "circleR0", true),
            ellipse_rx0: bool_param(params, "ellipseRX0", true),
            ellipse_ry0: bool_param(params, "ellipseRY0", true),
            rect_width0: bool_param(params, "rectWidth0", true),
            rect_height0: bool_param(params, "rectHeight0", true),
            pattern_width0: bool_param(params, "patternWidth0", true),
            pattern_height0: bool_param(params, "patternHeight0", true),
            image_width0: bool_param(params, "imageWidth0", true),
            image_height0: bool_param(params, "imageHeight0", true),
            path_empty_d: bool_param(params, "pathEmptyD", true),
            polyline_empty_points: bool_param(params, "polylineEmptyPoints", true),
            polygon_empty_points: bool_param(params, "polygonEmptyPoints", true),
        }
    }
}

struct GlobalState {
    all_references: HashSet<String>,
    deoptimized: bool,
}

fn collect_global_state(root: &XastRoot) -> GlobalState {
    let mut all_references = HashSet::<String>::new();
    let mut deoptimized = false;
    collect_global_state_children(&root.children, &mut all_references, &mut deoptimized);
    GlobalState {
        all_references,
        deoptimized,
    }
}

fn collect_global_state_children(
    children: &[XastChild],
    all_references: &mut HashSet<String>,
    deoptimized: &mut bool,
) {
    for child in children {
        let XastChild::Element(element) = child else {
            continue;
        };

        if (element.name == "style" && !element.children.is_empty()) || has_scripts(element) {
            *deoptimized = true;
        }

        for attribute in &element.attributes {
            for id in find_references(attribute.name.as_str(), attribute.value.as_str()) {
                all_references.insert(id);
            }
        }

        collect_global_state_children(&element.children, all_references, deoptimized);
    }
}

fn collect_all_references(root: &XastRoot) -> HashSet<String> {
    let mut all_references = HashSet::<String>::new();
    collect_global_state_children(&root.children, &mut all_references, &mut false);
    all_references
}

fn remove_hidden_elems(
    children: &mut Vec<XastChild>,
    parent_name: Option<&str>,
    stylesheet: &Stylesheet,
    all_references: &HashSet<String>,
    deoptimized: bool,
    options: RemoveHiddenElemsOptions,
    removed_def_ids: &mut HashSet<String>,
) {
    let mut index = 0usize;
    while index < children.len() {
        let mut removed = false;

        if let XastChild::Element(element) = &mut children[index] {
            let computed_style = compute_style(stylesheet, element);
            let should_remove = should_remove_element(
                element,
                &computed_style,
                all_references,
                deoptimized,
                options,
            );

            if should_remove {
                note_removed_def_id(element, parent_name, removed_def_ids);
                children.remove(index);
                removed = true;
            } else {
                remove_hidden_elems(
                    &mut element.children,
                    Some(element.name.as_str()),
                    stylesheet,
                    all_references,
                    deoptimized,
                    options,
                    removed_def_ids,
                );
            }
        }

        if !removed {
            index += 1;
        }
    }
}

fn remove_non_rendering_with_updated_references(
    children: &mut Vec<XastChild>,
    parent_name: Option<&str>,
    stylesheet: &Stylesheet,
    all_references: &HashSet<String>,
    options: RemoveHiddenElemsOptions,
) {
    let mut index = 0usize;
    while index < children.len() {
        let mut removed = false;

        if let XastChild::Element(element) = &mut children[index] {
            let computed_style = compute_style(stylesheet, element);
            let should_remove = (is_non_rendering_elem(element.name.as_str())
                && can_remove_non_rendering_node(element, all_references))
                || should_remove_opacity_zero_path(element, &computed_style, all_references, options);

            if should_remove {
                children.remove(index);
                removed = true;
            } else {
                remove_non_rendering_with_updated_references(
                    &mut element.children,
                    Some(element.name.as_str()),
                    stylesheet,
                    all_references,
                    options,
                );
            }
        }

        if !removed {
            index += 1;
        }
    }

    if parent_name == Some("defs") && children.is_empty() {}
}

#[allow(clippy::too_many_lines)]
fn should_remove_element(
    node: &XastElement,
    computed_style: &[(String, ComputedStyle)],
    all_references: &HashSet<String>,
    deoptimized: bool,
    options: RemoveHiddenElemsOptions,
) -> bool {
    if is_non_rendering_elem(node.name.as_str()) {
        return !deoptimized && can_remove_non_rendering_node(node, all_references);
    }

    if options.opacity0
        && static_style_value(computed_style, "opacity") == Some("0")
    {
        if node.name == "path" {
            return !deoptimized && can_remove_non_rendering_node(node, all_references);
        }
        return true;
    }

    if options.circle_r0
        && node.name == "circle"
        && node.children.is_empty()
        && node.get_attribute("r") == Some("0")
    {
        return true;
    }

    if options.ellipse_rx0
        && node.name == "ellipse"
        && node.children.is_empty()
        && node.get_attribute("rx") == Some("0")
    {
        return true;
    }

    if options.ellipse_ry0
        && node.name == "ellipse"
        && node.children.is_empty()
        && node.get_attribute("ry") == Some("0")
    {
        return true;
    }

    if options.rect_width0
        && node.name == "rect"
        && node.children.is_empty()
        && node.get_attribute("width") == Some("0")
    {
        return true;
    }

    if options.rect_height0
        && options.rect_width0
        && node.name == "rect"
        && node.children.is_empty()
        && node.get_attribute("height") == Some("0")
    {
        return true;
    }

    if options.pattern_width0
        && node.name == "pattern"
        && node.get_attribute("width") == Some("0")
    {
        return true;
    }

    if options.pattern_height0
        && node.name == "pattern"
        && node.get_attribute("height") == Some("0")
    {
        return true;
    }

    if options.image_width0
        && node.name == "image"
        && node.get_attribute("width") == Some("0")
    {
        return true;
    }

    if options.image_height0
        && node.name == "image"
        && node.get_attribute("height") == Some("0")
    {
        return true;
    }

    if options.polyline_empty_points
        && node.name == "polyline"
        && node.get_attribute("points").is_none()
    {
        return true;
    }

    if options.polygon_empty_points
        && node.name == "polygon"
        && node.get_attribute("points").is_none()
    {
        return true;
    }

    if options.is_hidden
        && static_style_value(computed_style, "visibility") == Some("hidden")
        && !has_descendant_visibility_visible(node)
    {
        return true;
    }

    if options.display_none
        && static_style_value(computed_style, "display") == Some("none")
        && node.name != "marker"
    {
        return true;
    }

    if options.path_empty_d && node.name == "path" {
        let Some(d) = node.get_attribute("d") else {
            return true;
        };
        let path_data = parse_path_data(d);
        if path_data.is_empty() {
            return true;
        }
        if path_data.len() == 1
            && static_style_value(computed_style, "marker-start").is_none()
            && static_style_value(computed_style, "marker-end").is_none()
        {
            return true;
        }
    }

    false
}

fn should_remove_opacity_zero_path(
    node: &XastElement,
    computed_style: &[(String, ComputedStyle)],
    all_references: &HashSet<String>,
    options: RemoveHiddenElemsOptions,
) -> bool {
    options.opacity0
        && node.name == "path"
        && static_style_value(computed_style, "opacity") == Some("0")
        && can_remove_non_rendering_node(node, all_references)
}

fn can_remove_non_rendering_node(node: &XastElement, all_references: &HashSet<String>) -> bool {
    if let Some(id) = node.get_attribute("id")
        && all_references.contains(id)
    {
        return false;
    }

    for child in &node.children {
        if let XastChild::Element(element) = child
            && !can_remove_non_rendering_node(element, all_references)
        {
            return false;
        }
    }

    true
}

fn static_style_value<'a>(
    computed_style: &'a [(String, ComputedStyle)],
    name: &str,
) -> Option<&'a str> {
    computed_style.iter().find_map(|(entry_name, value)| {
        if entry_name != name {
            return None;
        }
        match value {
            ComputedStyle::Static { value, .. } => Some(value.as_str()),
            ComputedStyle::Dynamic { .. } => None,
        }
    })
}

fn has_descendant_visibility_visible(node: &XastElement) -> bool {
    for child in &node.children {
        let XastChild::Element(element) = child else {
            continue;
        };

        if element.get_attribute("visibility") == Some("visible")
            || has_descendant_visibility_visible(element)
        {
            return true;
        }
    }
    false
}

fn note_removed_def_id(
    node: &XastElement,
    parent_name: Option<&str>,
    removed_def_ids: &mut HashSet<String>,
) {
    if parent_name == Some("defs")
        && let Some(id) = node.get_attribute("id")
    {
        removed_def_ids.insert(id.to_string());
    }
}

fn remove_use_nodes_referencing_ids(children: &mut Vec<XastChild>, removed_def_ids: &HashSet<String>) {
    let mut index = 0usize;
    while index < children.len() {
        let mut removed = false;
        if let XastChild::Element(element) = &mut children[index] {
            let is_removed_use = element.name == "use"
                && element.attributes.iter().any(|attribute| {
                    (attribute.name == "href" || attribute.name.ends_with(":href"))
                        && attribute
                            .value
                            .strip_prefix('#')
                            .is_some_and(|id| removed_def_ids.contains(id))
                });

            if is_removed_use {
                children.remove(index);
                removed = true;
            } else {
                remove_use_nodes_referencing_ids(&mut element.children, removed_def_ids);
            }
        }

        if !removed {
            index += 1;
        }
    }
}

fn remove_empty_defs(children: &mut Vec<XastChild>) {
    let mut index = 0usize;
    while index < children.len() {
        let mut removed = false;
        if let XastChild::Element(element) = &mut children[index] {
            remove_empty_defs(&mut element.children);
            if element.name == "defs" && element.children.is_empty() {
                children.remove(index);
                removed = true;
            }
        }

        if !removed {
            index += 1;
        }
    }
}

fn bool_param(params: Option<&Value>, name: &str, default: bool) -> bool {
    params
        .and_then(|value| value.get(name))
        .and_then(Value::as_bool)
        .unwrap_or(default)
}
