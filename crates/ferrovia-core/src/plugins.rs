use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use serde_json::Value;
use svgtypes::Color;

use crate::ast::{Attribute, Document, NodeKind, QuoteStyle};
use crate::config::{Config, PluginSpec};
use crate::error::{FerroviaError, Result};
use crate::geometry::{PathCommand, TransformOperation, parse_path_commands, parse_transform_operations};
use crate::style::{
    CssRule, StyleDeclaration, StylesheetRule, dedupe_declarations, parse_css_rules,
    parse_style_declarations, parse_stylesheet_rules, remove_declarations, selector_matches,
    selector_specificity, serialize_css_rules, serialize_minified_style_declarations,
    update_style_attribute,
};

const PRESET_DEFAULT: &[&str] = &[
    "removeDoctype",
    "removeXMLProcInst",
    "removeComments",
    "removeDeprecatedAttrs",
    "removeMetadata",
    "removeEditorsNSData",
    "cleanupAttrs",
    "mergeStyles",
    "inlineStyles",
    "minifyStyles",
    "cleanupIds",
    "removeUselessDefs",
    "cleanupNumericValues",
    "convertColors",
    "removeUnknownsAndDefaults",
    "removeNonInheritableGroupAttrs",
    "removeUselessStrokeAndFill",
    "cleanupEnableBackground",
    "removeHiddenElems",
    "removeEmptyText",
    "convertShapeToPath",
    "convertEllipseToCircle",
    "moveElemsAttrsToGroup",
    "moveGroupAttrsToElems",
    "collapseGroups",
    "convertPathData",
    "convertTransform",
    "removeEmptyAttrs",
    "removeEmptyContainers",
    "mergePaths",
    "removeUnusedNS",
    "sortAttrs",
    "sortDefsChildren",
    "removeDesc",
];

/// Apply the configured plugin pipeline to an already parsed document.
///
/// # Errors
///
/// Returns an error when the config references a plugin that is not implemented.
pub fn apply_plugins(doc: &mut Document, config: &Config) -> Result<()> {
    for plugin in expand_plugins(config) {
        let name = plugin.name().to_string();
        let params = plugin.params().cloned();
        match name.as_str() {
            "removeDoctype" => remove_by(doc, matches_doctype),
            "removeXMLProcInst" => remove_by(doc, matches_xml_decl),
            "removeComments" => remove_comments(doc, params.as_ref()),
            "removeDeprecatedAttrs" => remove_deprecated_attrs(doc, params.as_ref()),
            "removeMetadata" => remove_elements(doc, "metadata"),
            "removeEditorsNSData" => remove_editors_ns_data(doc, params.as_ref()),
            "cleanupAttrs" => cleanup_attrs(doc, params.as_ref()),
            "mergeStyles" => merge_styles(doc),
            "inlineStyles" => inline_styles(doc, params.as_ref()),
            "minifyStyles" => minify_styles(doc, params.as_ref()),
            "removeUselessDefs" => remove_useless_defs(doc),
            "cleanupNumericValues" => cleanup_numeric_values(doc, params.as_ref()),
            "convertColors" => convert_colors(doc, params.as_ref()),
            "removeUnknownsAndDefaults" => remove_unknowns_and_defaults(doc, params.as_ref()),
            "removeNonInheritableGroupAttrs" => remove_non_inheritable_group_attrs(doc),
            "removeUselessStrokeAndFill" => remove_useless_stroke_and_fill(doc, params.as_ref()),
            "cleanupEnableBackground" => cleanup_enable_background(doc),
            "removeHiddenElems" => remove_hidden_elems(doc, params.as_ref()),
            "removeEmptyText" => remove_empty_text(doc, params.as_ref()),
            "convertShapeToPath" => convert_shape_to_path(doc, params.as_ref()),
            "convertEllipseToCircle" => convert_ellipse_to_circle(doc),
            "convertTransform" => convert_transform(doc, params.as_ref()),
            "convertPathData" => convert_path_data(doc, params.as_ref()),
            "mergePaths" => merge_paths(doc, params.as_ref()),
            "moveElemsAttrsToGroup" => move_elems_attrs_to_group(doc),
            "moveGroupAttrsToElems" => move_group_attrs_to_elems(doc),
            "collapseGroups" => collapse_groups(doc),
            "removeEmptyAttrs" => remove_empty_attrs(doc),
            "removeEmptyContainers" => remove_empty_containers(doc),
            "removeUnusedNS" => remove_unused_ns(doc),
            "sortAttrs" => sort_attrs(doc, params.as_ref()),
            "sortDefsChildren" => sort_defs_children(doc),
            "removeTitle" => remove_elements(doc, "title"),
            "removeDesc" => remove_desc(doc, params.as_ref()),
            "cleanupIds" => cleanup_ids(doc, params.as_ref()),
            "removeDimensions" => remove_dimensions(doc),
            "removeXMLNS" => remove_xmlns(doc),
            other => return Err(FerroviaError::UnsupportedPlugin(other.to_string())),
        }
    }
    Ok(())
}

fn expand_plugins(config: &Config) -> Vec<PluginSpec> {
    let mut expanded = Vec::new();
    for plugin in &config.plugins {
        if !plugin.enabled() {
            continue;
        }
        if plugin.name() == "preset-default" {
            if let Some(params) = plugin.params() {
                let overrides = params
                    .get("overrides")
                    .and_then(Value::as_object)
                    .cloned()
                    .unwrap_or_default();
                for name in PRESET_DEFAULT {
                    match overrides.get(*name) {
                        Some(Value::Bool(false)) => {}
                        Some(Value::Object(object)) => {
                            expanded.push(PluginSpec::Configured(crate::config::PluginConfig {
                                name: (*name).to_string(),
                                params: Some(Value::Object(object.clone())),
                                enabled: true,
                            }));
                        }
                        _ => expanded.push(PluginSpec::Name((*name).to_string())),
                    }
                }
            } else {
                expanded.extend(
                    PRESET_DEFAULT
                        .iter()
                        .map(|name| PluginSpec::Name((*name).to_string())),
                );
            }
        } else {
            expanded.push(plugin.clone());
        }
    }
    expanded
}

fn remove_comments(doc: &mut Document, params: Option<&Value>) {
    let preserve_legal = params
        .and_then(|value| value.get("preservePatterns"))
        .and_then(Value::as_array)
        .is_some_and(|patterns| {
            patterns
                .iter()
                .any(|pattern| pattern.as_str() == Some("^!"))
        });

    remove_by(
        doc,
        |kind| matches!(kind, NodeKind::Comment(comment) if !(preserve_legal && comment.starts_with('!'))),
    );
}

fn cleanup_attrs(doc: &mut Document, params: Option<&Value>) {
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

    for node in &mut doc.nodes {
        let NodeKind::Element(element) = &mut node.kind else {
            continue;
        };
        for attribute in &mut element.attributes {
            if newlines {
                attribute.value = collapse_attribute_newlines(&attribute.value);
            }
            if trim {
                attribute.value = attribute.value.trim().to_string();
            }
            if spaces {
                attribute.value = collapse_repeating_spaces(&attribute.value);
            }
        }
    }
}

fn remove_useless_defs(doc: &mut Document) {
    let target_ids: Vec<_> = doc
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(id, node)| match &node.kind {
            NodeKind::Element(element)
                if element.name == "defs"
                    || (is_non_rendering(element.name.as_str())
                        && !element
                            .attributes
                            .iter()
                            .any(|attribute| attribute.name == "id")) =>
            {
                Some(id)
            }
            _ => None,
        })
        .collect();

    for target_id in target_ids {
        let parent_id = doc.node(target_id).parent;
        let mut useful_children = Vec::new();
        collect_useful_children(doc, target_id, &mut useful_children);

        if useful_children.is_empty() {
            detach_node(doc, target_id);
            continue;
        }

        let direct_children: Vec<_> = doc.children(target_id).collect();
        for child_id in &useful_children {
            if doc.node(*child_id).parent != Some(target_id) {
                detach_node(doc, *child_id);
            }
        }
        for child_id in direct_children {
            if !useful_children.contains(&child_id) {
                detach_node(doc, child_id);
            }
        }

        doc.reorder_children(target_id, &useful_children);
        doc.node_mut(target_id).parent = parent_id;
    }
}

fn remove_elements(doc: &mut Document, name: &str) {
    remove_by(
        doc,
        |kind| matches!(kind, NodeKind::Element(element) if element.name == name),
    );
}

fn cleanup_enable_background(doc: &mut Document) {
    let has_filter = doc
        .nodes
        .iter()
        .any(|node| matches!(&node.kind, NodeKind::Element(element) if element.name == "filter"));

    for node in &mut doc.nodes {
        let NodeKind::Element(element) = &mut node.kind else {
            continue;
        };

        let mut style_declarations = attribute_value(element.attributes.as_slice(), "style")
            .map(parse_style_declarations)
            .unwrap_or_default();
        dedupe_declarations(&mut style_declarations, "enable-background");

        if !has_filter {
            element
                .attributes
                .retain(|attribute| attribute.name != "enable-background");
            remove_declarations(&mut style_declarations, "enable-background");
            update_style_attribute(&mut element.attributes, &style_declarations);
            continue;
        }

        let width = attribute_value(element.attributes.as_slice(), "width").map(str::to_string);
        let height = attribute_value(element.attributes.as_slice(), "height").map(str::to_string);
        let has_dimensions = width.is_some() && height.is_some();
        if !matches!(element.name.as_str(), "svg" | "mask" | "pattern") || !has_dimensions {
            update_style_attribute(&mut element.attributes, &style_declarations);
            continue;
        }

        let width = width.unwrap_or_default();
        let height = height.unwrap_or_default();

        if let Some(attribute) =
            attribute_named_mut(element.attributes.as_mut_slice(), "enable-background")
        {
            match cleanup_enable_background_value(
                attribute.value.as_str(),
                element.name.as_str(),
                width.as_str(),
                height.as_str(),
            ) {
                Some(cleaned) => attribute.value = cleaned,
                None => attribute.value.clear(),
            }
        }
        element.attributes.retain(|attribute| {
            attribute.name != "enable-background" || !attribute.value.is_empty()
        });

        if let Some(index) = style_declarations
            .iter()
            .rposition(|declaration| declaration.name == "enable-background")
        {
            let current = style_declarations[index].value.clone();
            match cleanup_enable_background_value(
                current.as_str(),
                element.name.as_str(),
                width.as_str(),
                height.as_str(),
            ) {
                Some(cleaned) => style_declarations[index].value = cleaned,
                None => {
                    style_declarations.remove(index);
                }
            }
        }

        update_style_attribute(&mut element.attributes, &style_declarations);
    }
}

fn cleanup_enable_background_value(
    value: &str,
    node_name: &str,
    width: &str,
    height: &str,
) -> Option<String> {
    let parts: Vec<_> = value.split_whitespace().collect();
    if parts.len() == 5
        && parts[0] == "new"
        && parts[1] == "0"
        && parts[2] == "0"
        && parts[3] == width
        && parts[4] == height
    {
        return if node_name == "svg" {
            None
        } else {
            Some("new".to_string())
        };
    }
    Some(value.to_string())
}

#[derive(Debug, Clone, Copy)]
struct CleanupNumericValuesParams {
    float_precision: usize,
    leading_zero: bool,
    default_px: bool,
    convert_to_px: bool,
}

fn cleanup_numeric_values(doc: &mut Document, params: Option<&Value>) {
    let params = cleanup_numeric_values_params(params);
    for node in &mut doc.nodes {
        let NodeKind::Element(element) = &mut node.kind else {
            continue;
        };

        if let Some(viewbox) = attribute_named_mut(element.attributes.as_mut_slice(), "viewBox") {
            let cleaned = split_viewbox_values(viewbox.value.as_str())
                .into_iter()
                .map(|value| {
                    value.parse::<f64>().map_or(value, |number| {
                        round_number(number, params.float_precision).to_string()
                    })
                })
                .collect::<Vec<_>>()
                .join(" ");
            viewbox.value = cleaned;
        }

        for attribute in &mut element.attributes {
            if attribute.name == "version" {
                continue;
            }
            let Some((number, unit)) = parse_numeric_value(attribute.value.as_str()) else {
                continue;
            };

            let mut value = round_number(number, params.float_precision);
            let mut unit = unit.to_string();
            if params.convert_to_px
                && let Some(px_factor) = absolute_length_factor(unit.as_str())
            {
                let px_value = round_number(number * px_factor, params.float_precision);
                let candidate = format!("{px_value}px");
                if candidate.len() < attribute.value.len() {
                    value = px_value;
                    unit = "px".to_string();
                }
            }

            let mut serialized = if params.leading_zero {
                remove_leading_zero(value)
            } else {
                value.to_string()
            };
            if params.default_px && unit == "px" {
                unit.clear();
            }
            serialized.push_str(unit.as_str());
            attribute.value = serialized;
        }
    }
}

fn cleanup_numeric_values_params(params: Option<&Value>) -> CleanupNumericValuesParams {
    CleanupNumericValuesParams {
        float_precision: json_usize(params, "floatPrecision", 3),
        leading_zero: json_bool(params, "leadingZero", true),
        default_px: json_bool(params, "defaultPx", true),
        convert_to_px: json_bool(params, "convertToPx", true),
    }
}

fn split_viewbox_values(value: &str) -> Vec<String> {
    value
        .split(|char: char| char.is_ascii_whitespace() || char == ',')
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

fn parse_numeric_value(value: &str) -> Option<(f64, &str)> {
    let bytes = value.as_bytes();
    let mut index = 0;
    if matches!(bytes.first(), Some(b'+' | b'-')) {
        index += 1;
    }
    let mut saw_digit = false;
    while index < bytes.len() && char::from(bytes[index]).is_ascii_digit() {
        index += 1;
        saw_digit = true;
    }
    if bytes.get(index) == Some(&b'.') {
        index += 1;
        while index < bytes.len() && char::from(bytes[index]).is_ascii_digit() {
            index += 1;
            saw_digit = true;
        }
    }
    if !saw_digit {
        return None;
    }
    if matches!(bytes.get(index), Some(b'e' | b'E')) {
        let exponent_start = index;
        index += 1;
        if matches!(bytes.get(index), Some(b'+' | b'-')) {
            index += 1;
        }
        let exponent_digits_start = index;
        while index < bytes.len() && char::from(bytes[index]).is_ascii_digit() {
            index += 1;
        }
        if exponent_digits_start == index {
            index = exponent_start;
        }
    }
    let number = value[..index].parse::<f64>().ok()?;
    let unit = &value[index..];
    if !matches!(unit, "" | "%" | "px" | "pt" | "pc" | "mm" | "cm" | "m" | "in" | "ft" | "em" | "ex") {
        return None;
    }
    Some((number, unit))
}

fn absolute_length_factor(unit: &str) -> Option<f64> {
    match unit {
        "cm" => Some(96.0 / 2.54),
        "mm" => Some(96.0 / 25.4),
        "in" => Some(96.0),
        "pt" => Some(4.0 / 3.0),
        "pc" => Some(16.0),
        "px" => Some(1.0),
        _ => None,
    }
}

#[derive(Debug, Clone)]
enum CurrentColorMode {
    Disabled,
    Any,
    Exact(String),
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "SVGO convertColors parameters are boolean-heavy by design"
)]
#[derive(Debug, Clone)]
struct ConvertColorsParams {
    current_color: CurrentColorMode,
    names_to_hex: bool,
    rgb_to_hex: bool,
    convert_case: Option<ConvertCase>,
    shorthex: bool,
    shortname: bool,
}

#[derive(Debug, Clone, Copy)]
enum ConvertCase {
    Lower,
    Upper,
}

fn convert_colors(doc: &mut Document, params: Option<&Value>) {
    let params = convert_colors_params(params);
    for node_id in 1..doc.nodes.len() {
        let in_mask_subtree = has_ancestor_element(doc, node_id, "mask");
        let NodeKind::Element(element) = &mut doc.node_mut(node_id).kind else {
            continue;
        };
        for attribute in &mut element.attributes {
            if is_color_property(attribute.name.as_str()) {
                attribute.value =
                    convert_color_value(attribute.value.as_str(), &params, in_mask_subtree);
            }
        }
        if let Some(style_value) = attribute_value(element.attributes.as_slice(), "style") {
            let mut declarations = parse_style_declarations(style_value);
            for declaration in &mut declarations {
                if is_color_property(declaration.name.as_str()) {
                    declaration.value =
                        convert_color_value(declaration.value.as_str(), &params, in_mask_subtree);
                }
            }
            update_style_attribute(&mut element.attributes, &declarations);
        }
    }
}

fn convert_colors_params(params: Option<&Value>) -> ConvertColorsParams {
    let current_color = match params.and_then(|value| value.get("currentColor")) {
        Some(Value::Bool(true)) => CurrentColorMode::Any,
        Some(Value::String(text)) => CurrentColorMode::Exact(text.clone()),
        _ => CurrentColorMode::Disabled,
    };
    let convert_case = match params.and_then(|value| value.get("convertCase")) {
        Some(Value::Bool(false)) => None,
        Some(Value::String(value)) if value == "upper" => Some(ConvertCase::Upper),
        _ => Some(ConvertCase::Lower),
    };

    ConvertColorsParams {
        current_color,
        names_to_hex: json_bool(params, "names2hex", true),
        rgb_to_hex: json_bool(params, "rgb2hex", true),
        convert_case,
        shorthex: json_bool(params, "shorthex", true),
        shortname: json_bool(params, "shortname", true),
    }
}

#[expect(
    clippy::similar_names,
    reason = "Ellipse conversion is inherently paired around rx/ry semantics"
)]
fn convert_ellipse_to_circle(doc: &mut Document) {
    for node in &mut doc.nodes {
        let NodeKind::Element(element) = &mut node.kind else {
            continue;
        };
        if element.name != "ellipse" {
            continue;
        }
        let rx_attr = attribute_named(element.attributes.as_slice(), "rx").cloned();
        let ry_attr = attribute_named(element.attributes.as_slice(), "ry").cloned();
        let rx_raw = rx_attr
            .as_ref()
            .map_or("0", |attribute| attribute.value.as_str());
        let ry_raw = ry_attr
            .as_ref()
            .map_or("0", |attribute| attribute.value.as_str());
        if rx_raw != ry_raw && rx_raw != "auto" && ry_raw != "auto" {
            continue;
        }

        let radius = if rx_raw == "auto" { ry_raw } else { rx_raw };
        let quote = rx_attr
            .as_ref()
            .or(ry_attr.as_ref())
            .map_or(QuoteStyle::Double, |attribute| attribute.quote);

        element.name = "circle".to_string();
        element
            .attributes
            .retain(|attribute| attribute.name != "rx" && attribute.name != "ry");
        element.attributes.push(Attribute {
            name: "r".to_string(),
            value: radius.to_string(),
            quote,
        });
    }
}

#[derive(Debug, Clone, Copy)]
struct ConvertShapeToPathParams {
    convert_arcs: bool,
    float_precision: Option<usize>,
}

fn convert_shape_to_path(doc: &mut Document, params: Option<&Value>) {
    let params = convert_shape_to_path_params(params);
    let mut remove_ids = Vec::new();

    for node_id in 1..doc.nodes.len() {
        let Some(element_name) = node_element_name(doc, node_id).map(str::to_string) else {
            continue;
        };

        let replacement = match element_name.as_str() {
            "rect" => rect_to_path(node_element(doc, node_id), params),
            "line" => line_to_path(node_element(doc, node_id), params),
            "polyline" => poly_shape_to_path(node_element(doc, node_id), false, params),
            "polygon" => poly_shape_to_path(node_element(doc, node_id), true, params),
            "circle" if params.convert_arcs => circle_to_path(node_element(doc, node_id), params),
            "ellipse" if params.convert_arcs => ellipse_to_path(node_element(doc, node_id), params),
            _ => None,
        };

        let Some(replacement) = replacement else {
            continue;
        };
        match replacement {
            ShapePathRewrite::Replace(d) => {
                let NodeKind::Element(element) = &mut doc.node_mut(node_id).kind else {
                    continue;
                };
                element.name = "path".to_string();
                retain_non_shape_attributes(&mut element.attributes, element_name.as_str());
                element.attributes.push(Attribute {
                    name: "d".to_string(),
                    value: d,
                    quote: QuoteStyle::Double,
                });
            }
            ShapePathRewrite::Remove => remove_ids.push(node_id),
        }
    }

    for node_id in remove_ids {
        detach_node(doc, node_id);
    }
}

fn convert_shape_to_path_params(params: Option<&Value>) -> ConvertShapeToPathParams {
    ConvertShapeToPathParams {
        convert_arcs: json_bool(params, "convertArcs", false),
        float_precision: json_usize_opt(params, "floatPrecision"),
    }
}

enum ShapePathRewrite {
    Replace(String),
    Remove,
}

fn rect_to_path(
    element: Option<&crate::ast::Element>,
    params: ConvertShapeToPathParams,
) -> Option<ShapePathRewrite> {
    let element = element?;
    if attribute_value(element.attributes.as_slice(), "width").is_none()
        || attribute_value(element.attributes.as_slice(), "height").is_none()
        || attribute_value(element.attributes.as_slice(), "rx").is_some()
        || attribute_value(element.attributes.as_slice(), "ry").is_some()
    {
        return None;
    }
    let x = parse_plain_number(attribute_value(element.attributes.as_slice(), "x").unwrap_or("0"))?;
    let y = parse_plain_number(attribute_value(element.attributes.as_slice(), "y").unwrap_or("0"))?;
    let width = parse_plain_number(attribute_value(element.attributes.as_slice(), "width")?)?;
    let height = parse_plain_number(attribute_value(element.attributes.as_slice(), "height")?)?;
    Some(ShapePathRewrite::Replace(stringify_shape_path_data(
        &[
            ShapePathItem::new('M', vec![x, y]),
            ShapePathItem::new('H', vec![x + width]),
            ShapePathItem::new('V', vec![y + height]),
            ShapePathItem::new('H', vec![x]),
            ShapePathItem::new('z', Vec::new()),
        ],
        params.float_precision,
        false,
    )))
}

fn line_to_path(
    element: Option<&crate::ast::Element>,
    params: ConvertShapeToPathParams,
) -> Option<ShapePathRewrite> {
    let element = element?;
    let x1 = parse_plain_number(attribute_value(element.attributes.as_slice(), "x1").unwrap_or("0"))?;
    let y1 = parse_plain_number(attribute_value(element.attributes.as_slice(), "y1").unwrap_or("0"))?;
    let x2 = parse_plain_number(attribute_value(element.attributes.as_slice(), "x2").unwrap_or("0"))?;
    let y2 = parse_plain_number(attribute_value(element.attributes.as_slice(), "y2").unwrap_or("0"))?;
    Some(ShapePathRewrite::Replace(stringify_shape_path_data(
        &[
            ShapePathItem::new('M', vec![x1, y1]),
            ShapePathItem::new('L', vec![x2, y2]),
        ],
        params.float_precision,
        false,
    )))
}

fn poly_shape_to_path(
    element: Option<&crate::ast::Element>,
    close: bool,
    params: ConvertShapeToPathParams,
) -> Option<ShapePathRewrite> {
    let element = element?;
    let points = attribute_value(element.attributes.as_slice(), "points")?;
    let coords = extract_number_list(points);
    if coords.len() < 4 {
        return Some(ShapePathRewrite::Remove);
    }
    let mut items = Vec::new();
    for (index, chunk) in coords.chunks(2).enumerate() {
        if chunk.len() < 2 {
            break;
        }
        items.push(ShapePathItem::new(
            if index == 0 { 'M' } else { 'L' },
            vec![chunk[0], chunk[1]],
        ));
    }
    if close {
        items.push(ShapePathItem::new('z', Vec::new()));
    }
    Some(ShapePathRewrite::Replace(stringify_shape_path_data(
        &items,
        params.float_precision,
        false,
    )))
}

fn circle_to_path(
    element: Option<&crate::ast::Element>,
    params: ConvertShapeToPathParams,
) -> Option<ShapePathRewrite> {
    let element = element?;
    let cx = parse_plain_number(attribute_value(element.attributes.as_slice(), "cx").unwrap_or("0"))?;
    let cy = parse_plain_number(attribute_value(element.attributes.as_slice(), "cy").unwrap_or("0"))?;
    let r = parse_plain_number(attribute_value(element.attributes.as_slice(), "r").unwrap_or("0"))?;
    Some(ShapePathRewrite::Replace(stringify_shape_path_data(
        &[
            ShapePathItem::new('M', vec![cx, cy - r]),
            ShapePathItem::new('A', vec![r, r, 0.0, 1.0, 0.0, cx, cy + r]),
            ShapePathItem::new('A', vec![r, r, 0.0, 1.0, 0.0, cx, cy - r]),
            ShapePathItem::new('z', Vec::new()),
        ],
        params.float_precision,
        false,
    )))
}

fn ellipse_to_path(
    element: Option<&crate::ast::Element>,
    params: ConvertShapeToPathParams,
) -> Option<ShapePathRewrite> {
    let element = element?;
    let cx = parse_plain_number(attribute_value(element.attributes.as_slice(), "cx").unwrap_or("0"))?;
    let cy = parse_plain_number(attribute_value(element.attributes.as_slice(), "cy").unwrap_or("0"))?;
    let rx = parse_plain_number(attribute_value(element.attributes.as_slice(), "rx").unwrap_or("0"))?;
    let ry = parse_plain_number(attribute_value(element.attributes.as_slice(), "ry").unwrap_or("0"))?;
    Some(ShapePathRewrite::Replace(stringify_shape_path_data(
        &[
            ShapePathItem::new('M', vec![cx, cy - ry]),
            ShapePathItem::new('A', vec![rx, ry, 0.0, 1.0, 0.0, cx, cy + ry]),
            ShapePathItem::new('A', vec![rx, ry, 0.0, 1.0, 0.0, cx, cy - ry]),
            ShapePathItem::new('z', Vec::new()),
        ],
        params.float_precision,
        false,
    )))
}

#[derive(Debug, Clone)]
struct ShapePathItem {
    command: char,
    args: Vec<f64>,
}

impl ShapePathItem {
    const fn new(command: char, args: Vec<f64>) -> Self {
        Self { command, args }
    }
}

fn stringify_shape_path_data(
    items: &[ShapePathItem],
    precision: Option<usize>,
    disable_space_after_flags: bool,
) -> String {
    if items.is_empty() {
        return String::new();
    }
    if items.len() == 1 {
        let item = &items[0];
        return format!(
            "{}{}",
            item.command,
            stringify_shape_args(item.command, &item.args, precision, disable_space_after_flags)
        );
    }

    let mut result = String::new();
    let mut previous = items[0].clone();
    if items.get(1).is_some_and(|item| item.command == 'L') {
        previous.command = 'M';
    } else if items.get(1).is_some_and(|item| item.command == 'l') {
        previous.command = 'm';
    }

    for (index, item) in items.iter().enumerate().skip(1) {
        let merge = (previous.command == item.command && !matches!(previous.command, 'M' | 'm'))
            || (previous.command == 'M' && item.command == 'L')
            || (previous.command == 'm' && item.command == 'l');
        if merge {
            previous.args.extend(item.args.iter().copied());
            if index == items.len() - 1 {
                result.push(previous.command);
                result.push_str(
                    stringify_shape_args(
                        previous.command,
                        &previous.args,
                        precision,
                        disable_space_after_flags,
                    )
                    .as_str(),
                );
            }
            continue;
        }

        result.push(previous.command);
        result.push_str(
            stringify_shape_args(
                previous.command,
                &previous.args,
                precision,
                disable_space_after_flags,
            )
            .as_str(),
        );
        if index == items.len() - 1 {
            result.push(item.command);
            result.push_str(
                stringify_shape_args(
                    item.command,
                    &item.args,
                    precision,
                    disable_space_after_flags,
                )
                .as_str(),
            );
        } else {
            previous = item.clone();
        }
    }
    result
}

fn stringify_shape_args(
    command: char,
    args: &[f64],
    precision: Option<usize>,
    disable_space_after_flags: bool,
) -> String {
    let mut result = String::new();
    let mut previous = None;
    for (index, value) in args.iter().copied().enumerate() {
        let rounded = precision.map_or(value, |precision| round_number(value, precision));
        let rounded_str = remove_leading_zero(rounded);
        let disable_spaces = disable_space_after_flags
            && matches!(command, 'A' | 'a')
            && (index % 7 == 4 || index % 7 == 5);
        let needs_separator = !(disable_spaces
            || index == 0
            || rounded < 0.0
            || (previous.is_some_and(|previous: f64| previous.fract() != 0.0)
                && rounded_str.starts_with('.')));
        if needs_separator {
            result.push(' ');
        }
        result.push_str(rounded_str.as_str());
        previous = Some(rounded);
    }
    result
}

fn parse_plain_number(value: &str) -> Option<f64> {
    value.parse::<f64>().ok()
}

fn extract_number_list(value: &str) -> Vec<f64> {
    let bytes = value.as_bytes();
    let mut numbers = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        while index < bytes.len()
            && !matches!(bytes[index], b'+' | b'-' | b'.')
            && !char::from(bytes[index]).is_ascii_digit()
        {
            index += 1;
        }
        if index >= bytes.len() {
            break;
        }
        let start = index;
        if matches!(bytes[index], b'+' | b'-') {
            index += 1;
        }
        while index < bytes.len() && char::from(bytes[index]).is_ascii_digit() {
            index += 1;
        }
        if bytes.get(index) == Some(&b'.') {
            index += 1;
            while index < bytes.len() && char::from(bytes[index]).is_ascii_digit() {
                index += 1;
            }
        }
        if matches!(bytes.get(index), Some(b'e' | b'E')) {
            let exponent_start = index;
            index += 1;
            if matches!(bytes.get(index), Some(b'+' | b'-')) {
                index += 1;
            }
            let exponent_digits_start = index;
            while index < bytes.len() && char::from(bytes[index]).is_ascii_digit() {
                index += 1;
            }
            if exponent_digits_start == index {
                index = exponent_start;
            }
        }
        if let Ok(number) = value[start..index].parse::<f64>() {
            numbers.push(number);
        }
    }
    numbers
}

fn retain_non_shape_attributes(attributes: &mut Vec<Attribute>, element_name: &str) {
    attributes.retain(|attribute| {
        !matches!(
            (element_name, attribute.name.as_str()),
            ("rect", "x" | "y" | "width" | "height")
                | ("line", "x1" | "y1" | "x2" | "y2")
                | ("polyline" | "polygon", "points")
                | ("circle", "cx" | "cy" | "r")
                | ("ellipse", "cx" | "cy" | "rx" | "ry")
        )
    });
}

fn convert_color_value(value: &str, params: &ConvertColorsParams, in_mask_subtree: bool) -> String {
    let mut converted = value.to_string();

    if !in_mask_subtree {
        let current_color_matches = match &params.current_color {
            CurrentColorMode::Disabled => false,
            CurrentColorMode::Any => converted != "none",
            CurrentColorMode::Exact(expected) => converted == *expected,
        };
        if current_color_matches {
            return "currentColor".to_string();
        }
    }

    if params.names_to_hex
        && is_plain_color_name(converted.as_str())
        && let Ok(color) = Color::from_str(converted.as_str())
    {
        converted = serialize_long_hex(color, false);
    }

    if params.rgb_to_hex
        && is_rgb_function(converted.as_str())
        && let Ok(color) = Color::from_str(converted.as_str())
    {
        converted = serialize_long_hex(color, true);
    }

    if let Some(convert_case) = params.convert_case
        && !includes_url_reference(converted.as_str())
        && converted != "currentColor"
    {
        converted = match convert_case {
            ConvertCase::Lower => converted.to_lowercase(),
            ConvertCase::Upper => converted.to_uppercase(),
        };
    }

    if params.shorthex
        && let Some(shortened) = shorten_hex(converted.as_str())
    {
        converted = shortened;
    }

    if params.shortname
        && let Some(short_name) = short_color_name(converted.as_str())
    {
        converted = short_name.to_string();
    }

    converted
}

fn is_color_property(name: &str) -> bool {
    matches!(
        name,
        "color" | "fill" | "flood-color" | "lighting-color" | "stop-color" | "stroke"
    )
}

fn is_plain_color_name(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|char| char.is_ascii_alphabetic())
}

fn is_rgb_function(value: &str) -> bool {
    value.trim_start().starts_with("rgb(") || value.trim_start().starts_with("rgba(")
}

fn serialize_long_hex(color: Color, uppercase: bool) -> String {
    let mut serialized = format!("#{:02x}{:02x}{:02x}", color.red, color.green, color.blue);
    if uppercase {
        serialized.make_ascii_uppercase();
    }
    serialized
}

fn shorten_hex(value: &str) -> Option<String> {
    let hex = value.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let chars = hex.as_bytes();
    if chars[0] == chars[1] && chars[2] == chars[3] && chars[4] == chars[5] {
        return Some(format!(
            "#{}{}{}",
            char::from(chars[0]),
            char::from(chars[2]),
            char::from(chars[4])
        ));
    }
    None
}

fn short_color_name(value: &str) -> Option<&'static str> {
    match value.to_ascii_lowercase().as_str() {
        "#f0ffff" => Some("azure"),
        "#f5f5dc" => Some("beige"),
        "#ffe4c4" => Some("bisque"),
        "#a52a2a" => Some("brown"),
        "#ff7f50" => Some("coral"),
        "#ffd700" => Some("gold"),
        "#808080" => Some("gray"),
        "#008000" => Some("green"),
        "#4b0082" => Some("indigo"),
        "#fffff0" => Some("ivory"),
        "#f0e68c" => Some("khaki"),
        "#faf0e6" => Some("linen"),
        "#800000" => Some("maroon"),
        "#000080" => Some("navy"),
        "#808000" => Some("olive"),
        "#ffa500" => Some("orange"),
        "#da70d6" => Some("orchid"),
        "#cd853f" => Some("peru"),
        "#ffc0cb" => Some("pink"),
        "#dda0dd" => Some("plum"),
        "#800080" => Some("purple"),
        "#f00" | "#ff0000" => Some("red"),
        "#fa8072" => Some("salmon"),
        "#a0522d" => Some("sienna"),
        "#c0c0c0" => Some("silver"),
        "#fffafa" => Some("snow"),
        "#d2b48c" => Some("tan"),
        "#008080" => Some("teal"),
        "#ff6347" => Some("tomato"),
        "#ee82ee" => Some("violet"),
        "#f5deb3" => Some("wheat"),
        _ => None,
    }
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "SVGO convertTransform parameters are boolean-heavy by design"
)]
#[derive(Debug, Clone, Copy)]
struct ConvertTransformParams {
    convert_to_shorts: bool,
    deg_precision: Option<usize>,
    float_precision: usize,
    transform_precision: usize,
    matrix_to_transform: bool,
    short_translate: bool,
    short_scale: bool,
    short_rotate: bool,
    remove_useless: bool,
    collapse_into_one: bool,
    leading_zero: bool,
    negative_extra_space: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct TransformItem {
    name: &'static str,
    data: Vec<f64>,
}

enum TransformRewrite {
    KeepOriginal,
    Remove,
    Replace(String),
}

fn convert_transform(doc: &mut Document, params: Option<&Value>) {
    let params = convert_transform_params(params);
    for node_id in 1..doc.nodes.len() {
        let NodeKind::Element(element) = &mut doc.node_mut(node_id).kind else {
            continue;
        };

        for attribute_name in ["transform", "gradientTransform", "patternTransform"] {
            let Some(attribute) =
                attribute_named_mut(element.attributes.as_mut_slice(), attribute_name)
            else {
                continue;
            };
            match rewrite_transform(attribute.value.as_str(), params) {
                TransformRewrite::KeepOriginal => {}
                TransformRewrite::Remove => attribute.value.clear(),
                TransformRewrite::Replace(value) => attribute.value = value,
            }
        }
        element.attributes.retain(|attribute| {
            !matches!(
                attribute.name.as_str(),
                "transform" | "gradientTransform" | "patternTransform"
            ) || !attribute.value.is_empty()
        });
    }
}

fn convert_transform_params(params: Option<&Value>) -> ConvertTransformParams {
    ConvertTransformParams {
        convert_to_shorts: json_bool(params, "convertToShorts", true),
        deg_precision: json_usize_opt(params, "degPrecision"),
        float_precision: json_usize(params, "floatPrecision", 3),
        transform_precision: json_usize(params, "transformPrecision", 5),
        matrix_to_transform: json_bool(params, "matrixToTransform", true),
        short_translate: json_bool(params, "shortTranslate", true),
        short_scale: json_bool(params, "shortScale", true),
        short_rotate: json_bool(params, "shortRotate", true),
        remove_useless: json_bool(params, "removeUseless", true),
        collapse_into_one: json_bool(params, "collapseIntoOne", true),
        leading_zero: json_bool(params, "leadingZero", true),
        negative_extra_space: json_bool(params, "negativeExtraSpace", false),
    }
}

fn rewrite_transform(value: &str, params: ConvertTransformParams) -> TransformRewrite {
    let Ok(mut transforms) = parse_transform_items(value) else {
        return TransformRewrite::KeepOriginal;
    };
    if transforms.is_empty() {
        return TransformRewrite::Remove;
    }

    let params = define_transform_precision(&transforms, params);

    if params.collapse_into_one && transforms.len() > 1 {
        transforms = vec![transforms_multiply(&transforms)];
    }

    if params.convert_to_shorts {
        transforms = convert_to_shorts(transforms, params);
    } else {
        for transform in &mut transforms {
            round_transform_item(transform, params);
        }
    }

    if params.remove_useless {
        transforms = remove_useless_transforms(transforms);
    }

    if transforms.is_empty() {
        TransformRewrite::Remove
    } else {
        TransformRewrite::Replace(serialize_transforms(&transforms, params))
    }
}

fn parse_transform_items(value: &str) -> std::result::Result<Vec<TransformItem>, String> {
    parse_transform_operations(value).map(|operations| {
        operations
            .into_iter()
            .map(|operation| match operation {
                TransformOperation::Matrix { a, b, c, d, e, f } => TransformItem {
                    name: "matrix",
                    data: vec![a, b, c, d, e, f],
                },
                TransformOperation::Translate { tx, ty } => TransformItem {
                    name: "translate",
                    data: vec![tx, ty],
                },
                TransformOperation::Scale { sx, sy } => TransformItem {
                    name: "scale",
                    data: vec![sx, sy],
                },
                TransformOperation::Rotate { angle } => TransformItem {
                    name: "rotate",
                    data: vec![angle],
                },
                TransformOperation::SkewX { angle } => TransformItem {
                    name: "skewX",
                    data: vec![angle],
                },
                TransformOperation::SkewY { angle } => TransformItem {
                    name: "skewY",
                    data: vec![angle],
                },
            })
            .collect()
    })
}

#[expect(
    clippy::useless_let_if_seq,
    reason = "Port kept close to SVGO precision logic for easier parity review"
)]
#[expect(
    clippy::redundant_closure_for_method_calls,
    reason = "Port kept close to current Rust expression shape during spike"
)]
fn define_transform_precision(
    transforms: &[TransformItem],
    mut params: ConvertTransformParams,
) -> ConvertTransformParams {
    let matrix_values: Vec<_> = transforms
        .iter()
        .filter(|transform| transform.name == "matrix")
        .flat_map(|transform| transform.data.iter().copied().take(4))
        .collect();
    let mut number_of_digits = params.transform_precision;

    if !matrix_values.is_empty() {
        let matrix_precision = matrix_values
            .iter()
            .map(|value| float_digits(*value))
            .max()
            .unwrap_or(params.transform_precision);
        params.transform_precision = params.transform_precision.min(matrix_precision);
        number_of_digits = matrix_values
            .iter()
            .map(|value| value.to_string().chars().filter(|ch| ch.is_ascii_digit()).count())
            .max()
            .unwrap_or(params.transform_precision);
    }

    if params.deg_precision.is_none() {
        params.deg_precision = Some(params.float_precision.min(number_of_digits.saturating_sub(2)));
    }

    params
}

fn float_digits(value: f64) -> usize {
    let text = value.to_string();
    text.split_once('.').map_or(0, |(_, fraction)| fraction.len())
}

#[expect(
    clippy::float_cmp,
    reason = "Transform comparisons intentionally mirror SVGO semantics during the port"
)]
fn convert_to_shorts(
    mut transforms: Vec<TransformItem>,
    params: ConvertTransformParams,
) -> Vec<TransformItem> {
    let mut index = 0;
    while index < transforms.len() {
        if params.matrix_to_transform && transforms[index].name == "matrix" {
            let decomposed = matrix_to_transform(transforms[index].clone(), params);
            if serialize_transforms(&decomposed, params).len()
                <= serialize_transforms(&[transforms[index].clone()], params).len()
            {
                transforms.splice(index..=index, decomposed);
            }
        }

        round_transform_item(&mut transforms[index], params);

        if params.short_translate
            && transforms[index].name == "translate"
            && transforms[index].data.len() == 2
            && transforms[index].data[1] == 0.0
        {
            transforms[index].data.pop();
        }

        if params.short_scale
            && transforms[index].name == "scale"
            && transforms[index].data.len() == 2
            && transforms[index].data[0] == transforms[index].data[1]
        {
            transforms[index].data.pop();
        }

        if params.short_rotate
            && index >= 2
            && transforms[index - 2].name == "translate"
            && transforms[index - 1].name == "rotate"
            && transforms[index].name == "translate"
            && transforms[index - 2].data.len() == 2
            && transforms[index].data.len() == 2
            && transforms[index - 2].data[0] == -transforms[index].data[0]
            && transforms[index - 2].data[1] == -transforms[index].data[1]
        {
            let rotate = TransformItem {
                name: "rotate",
                data: vec![
                    transforms[index - 1].data[0],
                    transforms[index - 2].data[0],
                    transforms[index - 2].data[1],
                ],
            };
            transforms.splice(index - 2..=index, [rotate]);
            index = index.saturating_sub(2);
            continue;
        }

        index += 1;
    }

    transforms
}

fn remove_useless_transforms(transforms: Vec<TransformItem>) -> Vec<TransformItem> {
    transforms
        .into_iter()
        .filter(|transform| !is_identity_transform(transform))
        .collect()
}

#[expect(
    clippy::float_cmp,
    reason = "Identity detection intentionally mirrors SVGO transform logic during the port"
)]
fn is_identity_transform(transform: &TransformItem) -> bool {
    match transform.name {
        "rotate" | "skewX" | "skewY" => transform.data.first().copied().unwrap_or_default() == 0.0,
        "scale" => {
            transform.data.first().copied().unwrap_or(1.0) == 1.0
                && transform.data.get(1).copied().unwrap_or(1.0) == 1.0
        }
        "translate" => {
            transform.data.first().copied().unwrap_or_default() == 0.0
                && transform.data.get(1).copied().unwrap_or_default() == 0.0
        }
        "matrix" => transform.data == [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        _ => false,
    }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Ownership keeps the current port mechanically close to the SVGO helper flow"
)]
fn matrix_to_transform(
    matrix: TransformItem,
    params: ConvertTransformParams,
) -> Vec<TransformItem> {
    let mut shortest = vec![matrix.clone()];
    let mut shortest_len = serialize_transforms(&shortest, params).len();
    for decomposition in [decompose_qrab(&matrix), decompose_qrcd(&matrix)]
        .into_iter()
        .flatten()
    {
        let rounded: Vec<_> = decomposition
            .iter()
            .cloned()
            .map(|mut item| {
                round_transform_item(&mut item, params);
                item
            })
            .collect();
        let optimized = optimize_decomposition(&rounded, &decomposition);
        let length = serialize_transforms(&optimized, params).len();
        if length < shortest_len {
            shortest = optimized;
            shortest_len = length;
        }
    }
    shortest
}

#[expect(
    clippy::if_not_else,
    reason = "Branch structure stays aligned with the upstream optimization cases"
)]
fn optimize_decomposition(rounded: &[TransformItem], raw: &[TransformItem]) -> Vec<TransformItem> {
    let mut optimized = Vec::new();
    let mut index = 0;
    while index < rounded.len() {
        let transform = &rounded[index];
        if is_identity_transform(transform) {
            index += 1;
            continue;
        }
        match transform.name {
            "rotate" if matches!(transform.data[0], 180.0 | -180.0) => {
                if let Some(next) = rounded.get(index + 1)
                    && next.name == "scale"
                {
                    optimized.push(create_scale_transform(
                        next.data.iter().copied().map(|value| -value).collect(),
                    ));
                    index += 2;
                    continue;
                }
                optimized.push(TransformItem {
                    name: "scale",
                    data: vec![-1.0],
                });
            }
            "rotate" => {
                optimized.push(TransformItem {
                    name: "rotate",
                    data: if transform.data.get(1).copied().unwrap_or_default() != 0.0
                        || transform.data.get(2).copied().unwrap_or_default() != 0.0
                    {
                        transform.data[..3.min(transform.data.len())].to_vec()
                    } else {
                        transform.data[..1].to_vec()
                    },
                });
            }
            "scale" => optimized.push(create_scale_transform(transform.data.clone())),
            "skewX" | "skewY" => optimized.push(TransformItem {
                name: transform.name,
                data: vec![transform.data[0]],
            }),
            "translate" => {
                if let Some(next) = rounded.get(index + 1)
                    && next.name == "rotate"
                    && !matches!(next.data[0], 180.0 | -180.0 | 0.0)
                    && next.data.get(1).copied().unwrap_or_default() == 0.0
                    && next.data.get(2).copied().unwrap_or_default() == 0.0
                {
                    optimized.push(merge_translate_and_rotate(
                        raw[index].data[0],
                        raw[index].data[1],
                        raw[index + 1].data[0],
                    ));
                    index += 2;
                    continue;
                }
                optimized.push(TransformItem {
                    name: "translate",
                    data: if transform.data.get(1).copied().unwrap_or_default() != 0.0 {
                        transform.data[..2.min(transform.data.len())].to_vec()
                    } else {
                        transform.data[..1].to_vec()
                    },
                });
            }
            _ => optimized.push(transform.clone()),
        }
        index += 1;
    }
    if optimized.is_empty() {
        vec![TransformItem {
            name: "scale",
            data: vec![1.0],
        }]
    } else {
        optimized
    }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Owned transform data keeps the helper signature symmetric with its callers"
)]
#[expect(
    clippy::float_cmp,
    reason = "Scale shortening intentionally mirrors SVGO comparisons during the port"
)]
fn create_scale_transform(data: Vec<f64>) -> TransformItem {
    let keep_two = data.len() > 1 && data[0] != data[1];
    TransformItem {
        name: "scale",
        data: if keep_two {
            data[..2].to_vec()
        } else {
            vec![data[0]]
        },
    }
}

#[expect(
    clippy::many_single_char_names,
    reason = "Matrix coefficients follow the SVG a-f convention"
)]
#[expect(
    clippy::float_cmp,
    reason = "Decomposition comparisons intentionally mirror SVGO matrix logic during the port"
)]
#[expect(
    clippy::suboptimal_flops,
    reason = "Keeping the formulas visually close to SVGO improves auditability"
)]
fn decompose_qrab(matrix: &TransformItem) -> Option<Vec<TransformItem>> {
    let [a, b, c, d, e, f] = matrix_array(matrix);
    let delta = a * d - b * c;
    if delta == 0.0 {
        return None;
    }
    let r = a.hypot(b);
    if r == 0.0 {
        return None;
    }

    let mut decomposition = Vec::new();
    if e != 0.0 || f != 0.0 {
        decomposition.push(TransformItem {
            name: "translate",
            data: vec![e, f],
        });
    }

    let cos_angle = a / r;
    if cos_angle != 1.0 {
        let radians = cos_angle.acos();
        decomposition.push(TransformItem {
            name: "rotate",
            data: vec![
                radians_to_degrees(if b < 0.0 { -radians } else { radians }),
                0.0,
                0.0,
            ],
        });
    }

    let sx = r;
    let sy = delta / sx;
    if sx != 1.0 || sy != 1.0 {
        decomposition.push(TransformItem {
            name: "scale",
            data: vec![sx, sy],
        });
    }

    let ac_plus_bd = a * c + b * d;
    if ac_plus_bd != 0.0 {
        decomposition.push(TransformItem {
            name: "skewX",
            data: vec![radians_to_degrees((ac_plus_bd / (a * a + b * b)).atan())],
        });
    }

    Some(decomposition)
}

#[expect(
    clippy::many_single_char_names,
    reason = "Matrix coefficients follow the SVG a-f convention"
)]
#[expect(
    clippy::float_cmp,
    reason = "Decomposition comparisons intentionally mirror SVGO matrix logic during the port"
)]
#[expect(
    clippy::suboptimal_flops,
    reason = "Keeping the formulas visually close to SVGO improves auditability"
)]
fn decompose_qrcd(matrix: &TransformItem) -> Option<Vec<TransformItem>> {
    let [a, b, c, d, e, f] = matrix_array(matrix);
    let delta = a * d - b * c;
    if delta == 0.0 {
        return None;
    }
    let s = c.hypot(d);
    if s == 0.0 {
        return None;
    }

    let mut decomposition = Vec::new();
    if e != 0.0 || f != 0.0 {
        decomposition.push(TransformItem {
            name: "translate",
            data: vec![e, f],
        });
    }

    let radians =
        std::f64::consts::FRAC_PI_2 - if d < 0.0 { -1.0 } else { 1.0 } * (-c / s).acos();
    decomposition.push(TransformItem {
        name: "rotate",
        data: vec![radians_to_degrees(radians), 0.0, 0.0],
    });

    let sx = delta / s;
    let sy = s;
    if sx != 1.0 || sy != 1.0 {
        decomposition.push(TransformItem {
            name: "scale",
            data: vec![sx, sy],
        });
    }

    let ac_plus_bd = a * c + b * d;
    if ac_plus_bd != 0.0 {
        decomposition.push(TransformItem {
            name: "skewY",
            data: vec![radians_to_degrees((ac_plus_bd / (c * c + d * d)).atan())],
        });
    }

    Some(decomposition)
}

#[expect(
    clippy::suspicious_operation_groupings,
    reason = "Formula is a direct port of the SVGO rotation-center derivation"
)]
#[expect(
    clippy::suboptimal_flops,
    reason = "Keeping the formulas visually close to SVGO improves auditability"
)]
fn merge_translate_and_rotate(tx: f64, ty: f64, angle: f64) -> TransformItem {
    let radians = degrees_to_radians(angle);
    let d = 1.0 - radians.cos();
    let e = radians.sin();
    let cy = (d * ty + e * tx) / (d * d + e * e);
    let cx = (tx - e * cy) / d;
    TransformItem {
        name: "rotate",
        data: vec![angle, cx, cy],
    }
}

fn transforms_multiply(transforms: &[TransformItem]) -> TransformItem {
    let data = transforms
        .iter()
        .map(transform_to_matrix)
        .reduce(multiply_transform_matrices)
        .unwrap_or([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    TransformItem {
        name: "matrix",
        data: data.to_vec(),
    }
}

#[expect(
    clippy::suboptimal_flops,
    reason = "Keeping the formulas visually close to SVGO improves auditability"
)]
fn transform_to_matrix(transform: &TransformItem) -> [f64; 6] {
    match transform.name {
        "matrix" => matrix_array(transform),
        "translate" => [
            1.0,
            0.0,
            0.0,
            1.0,
            transform.data[0],
            transform.data.get(1).copied().unwrap_or_default(),
        ],
        "scale" => [
            transform.data[0],
            0.0,
            0.0,
            transform.data.get(1).copied().unwrap_or(transform.data[0]),
            0.0,
            0.0,
        ],
        "rotate" => {
            let cos = degrees_to_radians(transform.data[0]).cos();
            let sin = degrees_to_radians(transform.data[0]).sin();
            let cx = transform.data.get(1).copied().unwrap_or_default();
            let cy = transform.data.get(2).copied().unwrap_or_default();
            [
                cos,
                sin,
                -sin,
                cos,
                (1.0 - cos) * cx + sin * cy,
                (1.0 - cos) * cy - sin * cx,
            ]
        }
        "skewX" => [
            1.0,
            0.0,
            degrees_to_radians(transform.data[0]).tan(),
            1.0,
            0.0,
            0.0,
        ],
        "skewY" => [
            1.0,
            degrees_to_radians(transform.data[0]).tan(),
            0.0,
            1.0,
            0.0,
            0.0,
        ],
        _ => [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
    }
}

#[expect(
    clippy::suboptimal_flops,
    reason = "Keeping the formulas visually close to SVGO improves auditability"
)]
fn multiply_transform_matrices(a: [f64; 6], b: [f64; 6]) -> [f64; 6] {
    [
        a[0] * b[0] + a[2] * b[1],
        a[1] * b[0] + a[3] * b[1],
        a[0] * b[2] + a[2] * b[3],
        a[1] * b[2] + a[3] * b[3],
        a[0] * b[4] + a[2] * b[5] + a[4],
        a[1] * b[4] + a[3] * b[5] + a[5],
    ]
}

fn matrix_array(transform: &TransformItem) -> [f64; 6] {
    [
        transform.data[0],
        transform.data[1],
        transform.data[2],
        transform.data[3],
        transform.data[4],
        transform.data[5],
    ]
}

#[expect(
    clippy::or_fun_call,
    reason = "Current form keeps fallback precision logic explicit during the spike"
)]
fn round_transform_item(transform: &mut TransformItem, params: ConvertTransformParams) {
    match transform.name {
        "translate" => round_slice(&mut transform.data, params.float_precision),
        "rotate" => {
            let precision = params
                .deg_precision
                .unwrap_or(params.float_precision.saturating_sub(1));
            if let Some(angle) = transform.data.first_mut() {
                *angle = round_number(*angle, precision);
            }
            for value in transform.data.iter_mut().skip(1) {
                *value = round_number(*value, params.float_precision);
            }
        }
        "skewX" | "skewY" => {
            let precision = params
                .deg_precision
                .unwrap_or(params.float_precision.saturating_sub(1));
            round_slice(&mut transform.data, precision);
        }
        "scale" => round_slice(&mut transform.data, params.transform_precision),
        "matrix" => {
            for value in transform.data.iter_mut().take(4) {
                *value = round_number(*value, params.transform_precision);
            }
            for value in transform.data.iter_mut().skip(4) {
                *value = round_number(*value, params.float_precision);
            }
        }
        _ => {}
    }
}

fn round_slice(values: &mut [f64], precision: usize) {
    for value in values {
        *value = round_number(*value, precision);
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "Transform precision is bounded by plugin params and safe in practice"
)]
#[expect(
    clippy::cast_possible_wrap,
    reason = "Transform precision is bounded by plugin params and safe in practice"
)]
fn round_number(value: f64, precision: usize) -> f64 {
    if precision == 0 {
        return value.round();
    }
    let factor = 10_f64.powi(precision as i32);
    (value * factor).round() / factor
}

fn is_zero(value: f64) -> bool {
    value.abs() < 1e-12
}

#[expect(
    clippy::unnecessary_join,
    reason = "The explicit collect+join keeps the port shape close to SVGO serialization"
)]
fn serialize_transforms(transforms: &[TransformItem], params: ConvertTransformParams) -> String {
    transforms
        .iter()
        .cloned()
        .map(|mut transform| {
            round_transform_item(&mut transform, params);
            format!(
                "{}({})",
                transform.name,
                cleanup_out_data(&transform.data, params)
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

fn cleanup_out_data(data: &[f64], params: ConvertTransformParams) -> String {
    let mut output = String::new();
    let mut previous = None;
    for (index, value) in data.iter().copied().enumerate() {
        let mut delimiter = if index == 0 { "" } else { " " };
        let item = if params.leading_zero {
            remove_leading_zero(value)
        } else {
            value.to_string()
        };
        if params.negative_extra_space
            && !delimiter.is_empty()
            && (value < 0.0
                || (item.starts_with('.')
                    && previous.is_some_and(|prev: f64| prev.fract() != 0.0)))
        {
            delimiter = "";
        }
        output.push_str(delimiter);
        output.push_str(item.as_str());
        previous = Some(value);
    }
    output
}

fn remove_leading_zero(value: f64) -> String {
    let text = value.to_string();
    if value > 0.0 && value < 1.0 && text.starts_with('0') {
        text[1..].to_string()
    } else if value < 0.0 && value > -1.0 && text.as_bytes().get(1) == Some(&b'0') {
        format!("-{}", &text[2..])
    } else {
        text
    }
}

#[expect(
    clippy::suboptimal_flops,
    reason = "Direct arithmetic keeps the helper explicit and close to the source formulas"
)]
fn degrees_to_radians(value: f64) -> f64 {
    value * std::f64::consts::PI / 180.0
}

#[expect(
    clippy::suboptimal_flops,
    reason = "Direct arithmetic keeps the helper explicit and close to the source formulas"
)]
fn radians_to_degrees(value: f64) -> f64 {
    value * 180.0 / std::f64::consts::PI
}

fn json_bool(params: Option<&Value>, name: &str, default: bool) -> bool {
    params
        .and_then(|value| value.get(name))
        .and_then(Value::as_bool)
        .unwrap_or(default)
}

fn json_usize(params: Option<&Value>, name: &str, default: usize) -> usize {
    params
        .and_then(|value| value.get(name))
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(default)
}

fn json_usize_opt(params: Option<&Value>, name: &str) -> Option<usize> {
    params
        .and_then(|value| value.get(name))
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "SVGO convertPathData parameters are boolean-heavy by design"
)]
#[derive(Debug, Clone, Copy)]
struct ConvertPathDataParams {
    float_precision: usize,
    line_shorthands: bool,
    remove_useless: bool,
    collapse_repeated: bool,
    utilize_absolute: bool,
    leading_zero: bool,
    negative_extra_space: bool,
    no_space_after_flags: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct PathItem {
    command: char,
    args: Vec<f64>,
}

fn convert_path_data(doc: &mut Document, params: Option<&Value>) {
    let params = convert_path_data_params(params);
    let marker_mid_rules = collect_marker_mid_rules(doc);

    for node_id in 1..doc.nodes.len() {
        let has_marker_mid = path_has_marker_mid(doc, node_id, &marker_mid_rules);
        let NodeKind::Element(element) = &mut doc.node_mut(node_id).kind else {
            continue;
        };
        if element.name != "path" {
            continue;
        }
        let Some(attribute) = attribute_named_mut(element.attributes.as_mut_slice(), "d") else {
            continue;
        };

        let Ok(mut items) = parse_path_items(attribute.value.as_str()) else {
            continue;
        };
        if items.is_empty() {
            continue;
        }

        convert_path_to_relative(&mut items);
        if params.line_shorthands {
            convert_line_shorthands(&mut items);
        }
        if params.remove_useless && !has_marker_mid {
            remove_useless_path_items(&mut items);
        }
        if params.collapse_repeated && !has_marker_mid {
            collapse_repeated_path_items(&mut items);
        }
        if params.utilize_absolute && !has_marker_mid {
            utilize_absolute_path_items(&mut items, params);
        }

        attribute.value = serialize_path_items(&items, params);
    }
}

fn convert_path_data_params(params: Option<&Value>) -> ConvertPathDataParams {
    ConvertPathDataParams {
        float_precision: json_usize_opt(params, "floatPrecision").unwrap_or(3),
        line_shorthands: json_bool(params, "lineShorthands", true),
        remove_useless: json_bool(params, "removeUseless", true),
        collapse_repeated: json_bool(params, "collapseRepeated", true),
        utilize_absolute: json_bool(params, "utilizeAbsolute", true),
        leading_zero: json_bool(params, "leadingZero", true),
        negative_extra_space: json_bool(params, "negativeExtraSpace", true),
        no_space_after_flags: json_bool(params, "noSpaceAfterFlags", false),
    }
}

fn collect_marker_mid_rules(doc: &Document) -> Vec<String> {
    let mut selectors = Vec::new();
    for node_id in 1..doc.nodes.len() {
        let Some(element) = node_element(doc, node_id) else {
            continue;
        };
        if element.name != "style" {
            continue;
        }
        for child_id in doc.children(node_id) {
            let NodeKind::Text(text) = &doc.node(child_id).kind else {
                continue;
            };
            selectors.extend(
                parse_stylesheet_rules(text)
                    .into_iter()
                    .filter(|rule| rule.declarations.iter().any(|decl| decl.name == "marker-mid"))
                    .map(|rule| rule.selector),
            );
        }
    }
    selectors
}

fn path_has_marker_mid(doc: &Document, node_id: usize, rules: &[String]) -> bool {
    let Some(element) = node_element(doc, node_id) else {
        return false;
    };
    if element.name != "path" {
        return false;
    }
    attribute_value(element.attributes.as_slice(), "marker-mid").is_some()
        || attribute_value(element.attributes.as_slice(), "style")
            .is_some_and(|style| style.contains("marker-mid"))
        || rules
            .iter()
            .any(|selector| selector_matches(doc, node_id, selector))
}

fn parse_path_items(value: &str) -> std::result::Result<Vec<PathItem>, String> {
    parse_path_commands(value).map(|commands| commands.into_iter().map(PathItem::from).collect())
}

impl From<PathCommand> for PathItem {
    fn from(command: PathCommand) -> Self {
        match command {
            PathCommand::MoveTo { abs, x, y } => Self {
                command: if abs { 'M' } else { 'm' },
                args: vec![x, y],
            },
            PathCommand::LineTo { abs, x, y } => Self {
                command: if abs { 'L' } else { 'l' },
                args: vec![x, y],
            },
            PathCommand::HorizontalLineTo { abs, x } => Self {
                command: if abs { 'H' } else { 'h' },
                args: vec![x],
            },
            PathCommand::VerticalLineTo { abs, y } => Self {
                command: if abs { 'V' } else { 'v' },
                args: vec![y],
            },
            PathCommand::CurveTo {
                abs,
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => Self {
                command: if abs { 'C' } else { 'c' },
                args: vec![x1, y1, x2, y2, x, y],
            },
            PathCommand::SmoothCurveTo { abs, x2, y2, x, y } => Self {
                command: if abs { 'S' } else { 's' },
                args: vec![x2, y2, x, y],
            },
            PathCommand::Quadratic { abs, x1, y1, x, y } => Self {
                command: if abs { 'Q' } else { 'q' },
                args: vec![x1, y1, x, y],
            },
            PathCommand::SmoothQuadratic { abs, x, y } => Self {
                command: if abs { 'T' } else { 't' },
                args: vec![x, y],
            },
            PathCommand::EllipticalArc {
                abs,
                rx,
                ry,
                x_axis_rotation,
                large_arc,
                sweep,
                x,
                y,
            } => Self {
                command: if abs { 'A' } else { 'a' },
                args: vec![
                    rx,
                    ry,
                    x_axis_rotation,
                    if large_arc { 1.0 } else { 0.0 },
                    if sweep { 1.0 } else { 0.0 },
                    x,
                    y,
                ],
            },
            PathCommand::ClosePath { abs } => Self {
                command: if abs { 'Z' } else { 'z' },
                args: Vec::new(),
            },
        }
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "The relative-path rewrite stays readable as one SVGO-shaped normalization pass"
)]
fn convert_path_to_relative(items: &mut [PathItem]) {
    let mut cursor_x = 0.0;
    let mut cursor_y = 0.0;
    let mut start_x = 0.0;
    let mut start_y = 0.0;

    for (index, item) in items.iter_mut().enumerate() {
        match item.command {
            'M' => {
                let x = item.args[0];
                let y = item.args[1];
                if index != 0 {
                    item.command = 'm';
                    item.args[0] = x - cursor_x;
                    item.args[1] = y - cursor_y;
                }
                cursor_x = x;
                cursor_y = y;
                start_x = x;
                start_y = y;
            }
            'L' => {
                let x = item.args[0];
                let y = item.args[1];
                item.command = 'l';
                item.args[0] = x - cursor_x;
                item.args[1] = y - cursor_y;
                cursor_x = x;
                cursor_y = y;
            }
            'H' => {
                let x = item.args[0];
                item.command = 'h';
                item.args[0] = x - cursor_x;
                cursor_x = x;
            }
            'V' => {
                let y = item.args[0];
                item.command = 'v';
                item.args[0] = y - cursor_y;
                cursor_y = y;
            }
            'C' => {
                let x = item.args[4];
                let y = item.args[5];
                item.command = 'c';
                item.args[0] -= cursor_x;
                item.args[1] -= cursor_y;
                item.args[2] -= cursor_x;
                item.args[3] -= cursor_y;
                item.args[4] = x - cursor_x;
                item.args[5] = y - cursor_y;
                cursor_x = x;
                cursor_y = y;
            }
            'S' => {
                let x = item.args[2];
                let y = item.args[3];
                item.command = 's';
                item.args[0] -= cursor_x;
                item.args[1] -= cursor_y;
                item.args[2] = x - cursor_x;
                item.args[3] = y - cursor_y;
                cursor_x = x;
                cursor_y = y;
            }
            'Q' => {
                let x = item.args[2];
                let y = item.args[3];
                item.command = 'q';
                item.args[0] -= cursor_x;
                item.args[1] -= cursor_y;
                item.args[2] = x - cursor_x;
                item.args[3] = y - cursor_y;
                cursor_x = x;
                cursor_y = y;
            }
            'T' => {
                let x = item.args[0];
                let y = item.args[1];
                item.command = 't';
                item.args[0] = x - cursor_x;
                item.args[1] = y - cursor_y;
                cursor_x = x;
                cursor_y = y;
            }
            'A' => {
                let x = item.args[5];
                let y = item.args[6];
                item.command = 'a';
                item.args[5] = x - cursor_x;
                item.args[6] = y - cursor_y;
                cursor_x = x;
                cursor_y = y;
            }
            'm' => {
                cursor_x += item.args[0];
                cursor_y += item.args[1];
                start_x = cursor_x;
                start_y = cursor_y;
            }
            'l' | 't' => {
                cursor_x += item.args[0];
                cursor_y += item.args[1];
            }
            'h' => cursor_x += item.args[0],
            'v' => cursor_y += item.args[0],
            'c' => {
                cursor_x += item.args[4];
                cursor_y += item.args[5];
            }
            's' | 'q' => {
                cursor_x += item.args[2];
                cursor_y += item.args[3];
            }
            'a' => {
                cursor_x += item.args[5];
                cursor_y += item.args[6];
            }
            'Z' | 'z' => {
                cursor_x = start_x;
                cursor_y = start_y;
            }
            _ => {}
        }
    }
}

fn convert_line_shorthands(items: &mut [PathItem]) {
    for item in items {
        if item.command == 'l' && is_zero(item.args[0]) {
            item.command = 'v';
            item.args = vec![item.args[1]];
        } else if item.command == 'l' && is_zero(item.args[1]) {
            item.command = 'h';
            item.args = vec![item.args[0]];
        }
    }
}

fn remove_useless_path_items(items: &mut Vec<PathItem>) {
    items.retain(|item| match item.command {
        'l' => !is_zero(item.args[0]) || !is_zero(item.args[1]),
        'h' | 'v' => !is_zero(item.args[0]),
        _ => true,
    });
}

fn collapse_repeated_path_items(items: &mut Vec<PathItem>) {
    let mut collapsed: Vec<PathItem> = Vec::new();
    for item in items.drain(..) {
        if let Some(previous) = collapsed.last_mut()
            && previous.command == item.command
            && matches!(item.command, 'h' | 'v')
        {
            for value in item.args {
                if let Some(last) = previous.args.last_mut()
                    && has_same_sign(*last, value)
                {
                    *last += value;
                } else {
                    previous.args.push(value);
                }
            }
            continue;
        }
        collapsed.push(item);
    }
    *items = collapsed;
}

#[expect(
    clippy::too_many_lines,
    reason = "The mixed absolute-relative rewrite mirrors SVGO path heuristics in one pass"
)]
fn utilize_absolute_path_items(items: &mut [PathItem], params: ConvertPathDataParams) {
    let mut cursor_x = 0.0;
    let mut cursor_y = 0.0;
    let mut start_x = 0.0;
    let mut start_y = 0.0;

    for (index, item) in items.iter_mut().enumerate() {
        match item.command {
            'M' => {
                cursor_x = item.args[0];
                cursor_y = item.args[1];
                start_x = cursor_x;
                start_y = cursor_y;
            }
            'm' => {
                cursor_x += item.args[0];
                cursor_y += item.args[1];
                start_x = cursor_x;
                start_y = cursor_y;
            }
            'l' => {
                if item.args.len() != 2 {
                    cursor_x += *item.args.last().unwrap_or(&0.0);
                    cursor_y += *item.args.get(1).unwrap_or(&0.0);
                    continue;
                }
                let abs_x = cursor_x + item.args[0];
                let abs_y = cursor_y + item.args[1];
                let absolute_args = [abs_x, abs_y];
                if index > 0 && serialized_command_len('L', &absolute_args, params) < serialized_command_len('l', &item.args, params) {
                    item.command = 'L';
                    item.args[0] = abs_x;
                    item.args[1] = abs_y;
                }
                cursor_x = abs_x;
                cursor_y = abs_y;
            }
            'h' => {
                if item.args.len() != 1 {
                    cursor_x += item.args.iter().sum::<f64>();
                    continue;
                }
                let abs_x = cursor_x + item.args[0];
                let absolute_args = [abs_x];
                if serialized_command_len('H', &absolute_args, params)
                    < serialized_command_len('h', &item.args, params)
                {
                    item.command = 'H';
                    item.args[0] = abs_x;
                }
                cursor_x = abs_x;
            }
            'v' => {
                if item.args.len() != 1 {
                    cursor_y += item.args.iter().sum::<f64>();
                    continue;
                }
                let abs_y = cursor_y + item.args[0];
                let absolute_args = [abs_y];
                if serialized_command_len('V', &absolute_args, params)
                    < serialized_command_len('v', &item.args, params)
                {
                    item.command = 'V';
                    item.args[0] = abs_y;
                }
                cursor_y = abs_y;
            }
            'c' => {
                cursor_x += item.args[4];
                cursor_y += item.args[5];
            }
            's' | 'q' => {
                cursor_x += item.args[2];
                cursor_y += item.args[3];
            }
            't' => {
                cursor_x += item.args[0];
                cursor_y += item.args[1];
            }
            'a' => {
                cursor_x += item.args[5];
                cursor_y += item.args[6];
            }
            'L' | 'T' => {
                cursor_x = item.args[0];
                cursor_y = item.args[1];
            }
            'H' => cursor_x = item.args[0],
            'V' => cursor_y = item.args[0],
            'C' => {
                cursor_x = item.args[4];
                cursor_y = item.args[5];
            }
            'S' | 'Q' => {
                cursor_x = item.args[2];
                cursor_y = item.args[3];
            }
            'A' => {
                cursor_x = item.args[5];
                cursor_y = item.args[6];
            }
            'Z' | 'z' => {
                cursor_x = start_x;
                cursor_y = start_y;
            }
            _ => {}
        }
    }
}

fn serialized_command_len(command: char, args: &[f64], params: ConvertPathDataParams) -> usize {
    1 + serialize_path_numbers(
        args,
        params.float_precision,
        params.leading_zero,
        params.negative_extra_space,
        params.no_space_after_flags,
        matches!(command, 'a' | 'A'),
    )
    .len()
}

fn serialize_path_items(items: &[PathItem], params: ConvertPathDataParams) -> String {
    let mut output = String::new();
    for item in items {
        output.push(item.command);
        output.push_str(
            serialize_path_numbers(
                &item.args,
                params.float_precision,
                params.leading_zero,
                params.negative_extra_space,
                params.no_space_after_flags,
                matches!(item.command, 'a' | 'A'),
            )
            .as_str(),
        );
    }
    output
}

#[expect(
    clippy::fn_params_excessive_bools,
    reason = "Path serialization mirrors SVGO's boolean formatting switches"
)]
fn serialize_path_numbers(
    values: &[f64],
    precision: usize,
    leading_zero: bool,
    negative_extra_space: bool,
    no_space_after_flags: bool,
    is_arc: bool,
) -> String {
    let mut output = String::new();
    let mut previous = None;
    for (index, value) in values.iter().copied().enumerate() {
        let rounded = round_number(value, precision);
        let mut delimiter = if index == 0
            || (is_arc && no_space_after_flags && (index % 7 == 4 || index % 7 == 5))
        {
            ""
        } else {
            " "
        };
        let item = if leading_zero {
            remove_leading_zero(rounded)
        } else {
            rounded.to_string()
        };
        if negative_extra_space
            && !delimiter.is_empty()
            && (rounded < 0.0
                || (item.starts_with('.')
                    && previous.is_some_and(|prev: f64| prev.fract() != 0.0)))
        {
            delimiter = "";
        }
        output.push_str(delimiter);
        output.push_str(item.as_str());
        previous = Some(rounded);
    }
    output
}

fn has_same_sign(left: f64, right: f64) -> bool {
    is_zero(left) || is_zero(right) || left.is_sign_positive() == right.is_sign_positive()
}

#[derive(Debug, Clone, Copy)]
struct MergePathsParams {
    force: bool,
    float_precision: usize,
    no_space_after_flags: bool,
}

#[derive(Debug, Clone, Copy)]
struct PathBounds {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

fn merge_paths(doc: &mut Document, params: Option<&Value>) {
    let params = merge_paths_params(params);
    let stylesheet = collect_semantic_stylesheet(doc);
    let mut style_cache = HashMap::<usize, HashMap<String, String>>::new();
    let parent_ids: Vec<_> = doc
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(node_id, node)| {
            matches!(node.kind, NodeKind::Element(_)).then_some(node_id)
        })
        .collect();

    for parent_id in parent_ids {
        let child_ids: Vec<_> = doc.children(parent_id).collect();
        if child_ids.len() <= 1 {
            continue;
        }

        let mut remove_ids = Vec::new();
        let mut prev_child = child_ids[0];
        let mut prev_path_data: Option<Vec<PathItem>> = None;

        for &child_id in &child_ids[1..] {
            if !is_merge_path_candidate(doc, prev_child) {
                if let Some(data) = prev_path_data.take() {
                    write_merged_path_data(doc, prev_child, &data, params);
                }
                prev_child = child_id;
                continue;
            }

            if !is_merge_path_candidate(doc, child_id)
                || merge_paths_style_deopt(doc, child_id, &stylesheet, &mut style_cache)
                || !merge_path_attributes_match(doc, prev_child, child_id)
            {
                if let Some(data) = prev_path_data.take() {
                    write_merged_path_data(doc, prev_child, &data, params);
                }
                prev_child = child_id;
                continue;
            }

            let Some(current_path_data) = parse_node_path_data(doc, child_id) else {
                if let Some(data) = prev_path_data.take() {
                    write_merged_path_data(doc, prev_child, &data, params);
                }
                prev_child = child_id;
                continue;
            };

            if prev_path_data.is_none() {
                prev_path_data = parse_node_path_data(doc, prev_child);
            }
            let Some(previous_path_data) = &mut prev_path_data else {
                prev_child = child_id;
                continue;
            };

            if params.force || !paths_intersect(previous_path_data, &current_path_data) {
                previous_path_data.extend(current_path_data);
                remove_ids.push(child_id);
                continue;
            }

            if let Some(data) = prev_path_data.take() {
                write_merged_path_data(doc, prev_child, &data, params);
            }
            prev_child = child_id;
        }

        if let Some(data) = prev_path_data.take() {
            write_merged_path_data(doc, prev_child, &data, params);
        }

        for node_id in remove_ids {
            detach_node(doc, node_id);
        }
    }
}

fn merge_paths_params(params: Option<&Value>) -> MergePathsParams {
    MergePathsParams {
        force: json_bool(params, "force", false),
        float_precision: json_usize_opt(params, "floatPrecision").unwrap_or(3),
        no_space_after_flags: json_bool(params, "noSpaceAfterFlags", false),
    }
}

fn is_merge_path_candidate(doc: &Document, node_id: usize) -> bool {
    let Some(element) = node_element(doc, node_id) else {
        return false;
    };
    element.name == "path"
        && doc.children(node_id).next().is_none()
        && attribute_value(element.attributes.as_slice(), "d").is_some()
}

fn merge_paths_style_deopt(
    doc: &Document,
    node_id: usize,
    stylesheet: &[StylesheetRule],
    cache: &mut HashMap<usize, HashMap<String, String>>,
) -> bool {
    let computed_style = compute_static_style(doc, node_id, stylesheet, cache);
    ["marker-start", "marker-mid", "marker-end", "clip-path", "mask", "mask-image"]
        .iter()
        .any(|name| computed_style.contains_key(*name))
        || ["fill", "filter", "stroke"].iter().any(|name| {
            computed_style
                .get(*name)
                .is_some_and(|value| includes_url_reference(value))
        })
}

fn merge_path_attributes_match(doc: &Document, left_id: usize, right_id: usize) -> bool {
    let Some(left) = node_element(doc, left_id) else {
        return false;
    };
    let Some(right) = node_element(doc, right_id) else {
        return false;
    };
    if left.attributes.len() != right.attributes.len() {
        return false;
    }
    right
        .attributes
        .iter()
        .filter(|attribute| attribute.name != "d")
        .all(|attribute| {
            attribute_value(left.attributes.as_slice(), attribute.name.as_str())
                == Some(attribute.value.as_str())
        })
}

fn parse_node_path_data(doc: &Document, node_id: usize) -> Option<Vec<PathItem>> {
    node_element(doc, node_id)
        .and_then(|element| attribute_value(element.attributes.as_slice(), "d"))
        .and_then(|value| parse_path_items(value).ok())
}

fn write_merged_path_data(
    doc: &mut Document,
    node_id: usize,
    items: &[PathItem],
    params: MergePathsParams,
) {
    let Some(element) = node_element(doc, node_id) else {
        return;
    };
    let quote = attribute_named(element.attributes.as_slice(), "d")
        .map_or(QuoteStyle::Double, |attribute| attribute.quote);
    let serialized = serialize_path_items(
        items,
        ConvertPathDataParams {
            float_precision: params.float_precision,
            line_shorthands: false,
            remove_useless: false,
            collapse_repeated: false,
            utilize_absolute: false,
            leading_zero: true,
            negative_extra_space: true,
            no_space_after_flags: params.no_space_after_flags,
        },
    );
    let NodeKind::Element(element) = &mut doc.node_mut(node_id).kind else {
        return;
    };
    set_or_push_attribute(&mut element.attributes, "d", serialized.as_str(), quote);
}

fn paths_intersect(left: &[PathItem], right: &[PathItem]) -> bool {
    let Some(left_bounds) = path_bounds(left) else {
        return true;
    };
    let Some(right_bounds) = path_bounds(right) else {
        return true;
    };
    !(left_bounds.max_x <= right_bounds.min_x
        || right_bounds.max_x <= left_bounds.min_x
        || left_bounds.max_y <= right_bounds.min_y
        || right_bounds.max_y <= left_bounds.min_y)
}

#[expect(
    clippy::too_many_lines,
    reason = "Bounding-box sampling keeps mergePaths conservative without a separate geometry module"
)]
fn path_bounds(items: &[PathItem]) -> Option<PathBounds> {
    let mut bounds = None;
    let mut cursor_x = 0.0;
    let mut cursor_y = 0.0;
    let mut start_x = 0.0;
    let mut start_y = 0.0;

    for item in items {
        match item.command {
            'M' => {
                cursor_x = item.args[0];
                cursor_y = item.args[1];
                start_x = cursor_x;
                start_y = cursor_y;
                include_point(&mut bounds, cursor_x, cursor_y);
            }
            'm' => {
                cursor_x += item.args[0];
                cursor_y += item.args[1];
                start_x = cursor_x;
                start_y = cursor_y;
                include_point(&mut bounds, cursor_x, cursor_y);
            }
            'L' | 'T' => {
                cursor_x = item.args[0];
                cursor_y = item.args[1];
                include_point(&mut bounds, cursor_x, cursor_y);
            }
            'l' | 't' => {
                cursor_x += item.args[0];
                cursor_y += item.args[1];
                include_point(&mut bounds, cursor_x, cursor_y);
            }
            'H' => {
                cursor_x = item.args[0];
                include_point(&mut bounds, cursor_x, cursor_y);
            }
            'h' => {
                cursor_x += item.args[0];
                include_point(&mut bounds, cursor_x, cursor_y);
            }
            'V' => {
                cursor_y = item.args[0];
                include_point(&mut bounds, cursor_x, cursor_y);
            }
            'v' => {
                cursor_y += item.args[0];
                include_point(&mut bounds, cursor_x, cursor_y);
            }
            'C' => {
                include_point(&mut bounds, item.args[0], item.args[1]);
                include_point(&mut bounds, item.args[2], item.args[3]);
                cursor_x = item.args[4];
                cursor_y = item.args[5];
                include_point(&mut bounds, cursor_x, cursor_y);
            }
            'c' => {
                include_point(&mut bounds, cursor_x + item.args[0], cursor_y + item.args[1]);
                include_point(&mut bounds, cursor_x + item.args[2], cursor_y + item.args[3]);
                cursor_x += item.args[4];
                cursor_y += item.args[5];
                include_point(&mut bounds, cursor_x, cursor_y);
            }
            'S' | 'Q' => {
                include_point(&mut bounds, item.args[0], item.args[1]);
                cursor_x = item.args[2];
                cursor_y = item.args[3];
                include_point(&mut bounds, cursor_x, cursor_y);
            }
            's' | 'q' => {
                include_point(&mut bounds, cursor_x + item.args[0], cursor_y + item.args[1]);
                cursor_x += item.args[2];
                cursor_y += item.args[3];
                include_point(&mut bounds, cursor_x, cursor_y);
            }
            'A' => {
                let rx = item.args[0].abs();
                let ry = item.args[1].abs();
                include_point(&mut bounds, cursor_x - rx, cursor_y - ry);
                include_point(&mut bounds, cursor_x + rx, cursor_y + ry);
                cursor_x = item.args[5];
                cursor_y = item.args[6];
                include_point(&mut bounds, cursor_x - rx, cursor_y - ry);
                include_point(&mut bounds, cursor_x + rx, cursor_y + ry);
            }
            'a' => {
                let rx = item.args[0].abs();
                let ry = item.args[1].abs();
                include_point(&mut bounds, cursor_x - rx, cursor_y - ry);
                include_point(&mut bounds, cursor_x + rx, cursor_y + ry);
                cursor_x += item.args[5];
                cursor_y += item.args[6];
                include_point(&mut bounds, cursor_x - rx, cursor_y - ry);
                include_point(&mut bounds, cursor_x + rx, cursor_y + ry);
            }
            'Z' | 'z' => {
                cursor_x = start_x;
                cursor_y = start_y;
                include_point(&mut bounds, cursor_x, cursor_y);
            }
            _ => {}
        }
    }

    bounds
}

#[expect(
    clippy::missing_const_for_fn,
    reason = "The helper mutates floating-point bounds; const adds no practical value here"
)]
fn include_point(bounds: &mut Option<PathBounds>, x: f64, y: f64) {
    match bounds {
        Some(bounds) => {
            bounds.min_x = bounds.min_x.min(x);
            bounds.min_y = bounds.min_y.min(y);
            bounds.max_x = bounds.max_x.max(x);
            bounds.max_y = bounds.max_y.max(y);
        }
        None => {
            *bounds = Some(PathBounds {
                min_x: x,
                min_y: y,
                max_x: x,
                max_y: y,
            });
        }
    }
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "SVGO removeUnknownsAndDefaults parameters are boolean-heavy by design"
)]
#[derive(Debug, Clone, Copy)]
struct RemoveUnknownsAndDefaultsParams {
    unknown_content: bool,
    unknown_attrs: bool,
    default_attrs: bool,
    default_markup_declarations: bool,
    useless_overrides: bool,
    keep_data_attrs: bool,
    keep_aria_attrs: bool,
    keep_role_attr: bool,
}

fn remove_unknowns_and_defaults(doc: &mut Document, params: Option<&Value>) {
    let params = remove_unknowns_and_defaults_params(params);
    if params.default_markup_declarations {
        for node in &mut doc.nodes {
            let NodeKind::XmlDecl(decl) = &mut node.kind else {
                continue;
            };
            decl.attributes
                .retain(|attribute| !(attribute.name == "standalone" && attribute.value == "no"));
        }
    }

    let stylesheet = collect_semantic_stylesheet(doc);
    let mut computed_styles = HashMap::<usize, HashMap<String, String>>::new();

    for node_id in 1..doc.nodes.len() {
        if is_in_foreign_object_subtree(doc, node_id) {
            continue;
        }

        let Some(parent_id) = doc.node(node_id).parent else {
            continue;
        };
        let Some(element_name) = node_element_name(doc, node_id).map(str::to_string) else {
            continue;
        };
        if element_name.contains(':') || element_name == "foreignObject" {
            continue;
        }

        if params.unknown_content && should_remove_unknown_child(doc, parent_id, element_name.as_str()) {
            detach_node(doc, node_id);
            continue;
        }

        let has_id = node_has_id(doc, node_id);
        let computed_parent_style = if matches!(doc.node(parent_id).kind, NodeKind::Element(_)) {
            compute_static_style(doc, parent_id, &stylesheet, &mut computed_styles)
        } else {
            HashMap::new()
        };

        let NodeKind::Element(element) = &mut doc.node_mut(node_id).kind else {
            continue;
        };
        if !has_attribute_model(element.name.as_str()) {
            continue;
        }

        element.attributes.retain(|attribute| {
            if params.keep_data_attrs && attribute.name.starts_with("data-") {
                return true;
            }
            if params.keep_aria_attrs && attribute.name.starts_with("aria-") {
                return true;
            }
            if params.keep_role_attr && attribute.name == "role" {
                return true;
            }
            if attribute.name == "xmlns" {
                return true;
            }
            if let Some((prefix, _)) = attribute.name.split_once(':')
                && prefix != "xml"
                && prefix != "xlink"
            {
                return true;
            }

            if params.unknown_attrs
                && !attribute_allowed_on_element(element.name.as_str(), attribute.name.as_str())
            {
                return false;
            }

            if params.default_attrs
                && !has_id
                && attribute_default_value(element.name.as_str(), attribute.name.as_str())
                    == Some(attribute.value.as_str())
                && !computed_parent_style.contains_key(attribute.name.as_str())
                && !stylesheet_mentions_attr_selector(&stylesheet, attribute.name.as_str())
            {
                return false;
            }

            if params.useless_overrides
                && !has_id
                && !is_non_inheritable_group_presentation_attr(attribute.name.as_str())
                && computed_parent_style
                    .get(attribute.name.as_str())
                    .is_some_and(|value| value == &attribute.value)
            {
                return false;
            }

            true
        });
    }
}

fn remove_unknowns_and_defaults_params(params: Option<&Value>) -> RemoveUnknownsAndDefaultsParams {
    RemoveUnknownsAndDefaultsParams {
        unknown_content: json_bool(params, "unknownContent", true),
        unknown_attrs: json_bool(params, "unknownAttrs", true),
        default_attrs: json_bool(params, "defaultAttrs", true),
        default_markup_declarations: json_bool(params, "defaultMarkupDeclarations", true),
        useless_overrides: json_bool(params, "uselessOverrides", true),
        keep_data_attrs: json_bool(params, "keepDataAttrs", true),
        keep_aria_attrs: json_bool(params, "keepAriaAttrs", true),
        keep_role_attr: json_bool(params, "keepRoleAttr", false),
    }
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "SVGO removeHiddenElems parameters are boolean-heavy by design"
)]
#[derive(Debug, Clone, Copy)]
struct RemoveHiddenElemsParams {
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

#[expect(
    clippy::too_many_lines,
    reason = "The hidden-element pass stays readable as one SVGO-shaped orchestration pass"
)]
fn remove_hidden_elems(doc: &mut Document, params: Option<&Value>) {
    let params = remove_hidden_elems_params(params);
    let stylesheet = collect_semantic_stylesheet(doc);
    let mut computed_styles = HashMap::<usize, HashMap<String, String>>::new();
    let reference_ids = collect_reference_ids(doc);
    let deoptimized_non_rendering = hidden_elems_deoptimized(doc);
    let use_references = collect_use_references(doc);
    let mut removed_def_ids = HashSet::<String>::new();
    let mut defs_nodes = Vec::<usize>::new();
    let mut delayed_non_rendering = Vec::<usize>::new();

    for node_id in 1..doc.nodes.len() {
        let Some(element_name) = node_element_name(doc, node_id).map(str::to_string) else {
            continue;
        };

        if element_name == "defs" {
            defs_nodes.push(node_id);
        }

        if is_non_rendering(element_name.as_str()) {
            delayed_non_rendering.push(node_id);
            continue;
        }

        let computed_style = compute_static_style(doc, node_id, &stylesheet, &mut computed_styles);
        let Some(element) = node_element(doc, node_id) else {
            continue;
        };

        if params.opacity0
            && computed_style.get("opacity").is_some_and(|value| value == "0")
        {
            if element_name == "path" {
                delayed_non_rendering.push(node_id);
                continue;
            }
            record_removed_def_id(doc, node_id, &mut removed_def_ids);
            detach_node(doc, node_id);
            continue;
        }

        let should_remove = match element_name.as_str() {
            "circle" if params.circle_r0 && doc.children(node_id).next().is_none() => {
                attribute_value(element.attributes.as_slice(), "r") == Some("0")
            }
            "ellipse" if doc.children(node_id).next().is_none() => {
                (params.ellipse_rx0
                    && attribute_value(element.attributes.as_slice(), "rx") == Some("0"))
                    || (params.ellipse_ry0
                        && attribute_value(element.attributes.as_slice(), "ry") == Some("0"))
            }
            "rect" if doc.children(node_id).next().is_none() => {
                (params.rect_width0
                    && attribute_value(element.attributes.as_slice(), "width") == Some("0"))
                    || (params.rect_height0
                        && attribute_value(element.attributes.as_slice(), "height") == Some("0"))
            }
            "pattern" => {
                (params.pattern_width0
                    && attribute_value(element.attributes.as_slice(), "width") == Some("0"))
                    || (params.pattern_height0
                        && attribute_value(element.attributes.as_slice(), "height") == Some("0"))
            }
            "image" => {
                (params.image_width0
                    && attribute_value(element.attributes.as_slice(), "width") == Some("0"))
                    || (params.image_height0
                        && attribute_value(element.attributes.as_slice(), "height") == Some("0"))
            }
            "path" if params.path_empty_d => {
                should_remove_path(element.attributes.as_slice(), &computed_style)
            }
            "polyline" if params.polyline_empty_points => {
                attribute_value(element.attributes.as_slice(), "points").is_none()
            }
            "polygon" if params.polygon_empty_points => {
                attribute_value(element.attributes.as_slice(), "points").is_none()
            }
            _ => false,
        } || (params.is_hidden
            && computed_style
                .get("visibility")
                .is_some_and(|value| value == "hidden")
            && !has_visible_descendant_attr(doc, node_id))
            || (params.display_none
                && element_name != "marker"
                && computed_style
                    .get("display")
                    .is_some_and(|value| value == "none"));

        if should_remove {
            record_removed_def_id(doc, node_id, &mut removed_def_ids);
            detach_node(doc, node_id);
        }
    }

    if !deoptimized_non_rendering {
        for node_id in delayed_non_rendering {
            if doc.node(node_id).parent.is_none() {
                continue;
            }
            if can_remove_non_rendering_node(doc, node_id, &reference_ids) {
                record_removed_def_id(doc, node_id, &mut removed_def_ids);
                detach_node(doc, node_id);
            }
        }
    }

    for id in removed_def_ids {
        if let Some(use_nodes) = use_references.get(id.as_str()) {
            for &use_id in use_nodes {
                if doc.node(use_id).parent.is_some() {
                    detach_node(doc, use_id);
                }
            }
        }
    }

    for defs_id in defs_nodes {
        if doc.node(defs_id).parent.is_some() && doc.children(defs_id).next().is_none() {
            detach_node(doc, defs_id);
        }
    }
}

fn remove_hidden_elems_params(params: Option<&Value>) -> RemoveHiddenElemsParams {
    RemoveHiddenElemsParams {
        is_hidden: json_bool(params, "isHidden", true),
        display_none: json_bool(params, "displayNone", true),
        opacity0: json_bool(params, "opacity0", true),
        circle_r0: json_bool(params, "circleR0", true),
        ellipse_rx0: json_bool(params, "ellipseRX0", true),
        ellipse_ry0: json_bool(params, "ellipseRY0", true),
        rect_width0: json_bool(params, "rectWidth0", true),
        rect_height0: json_bool(params, "rectHeight0", true),
        pattern_width0: json_bool(params, "patternWidth0", true),
        pattern_height0: json_bool(params, "patternHeight0", true),
        image_width0: json_bool(params, "imageWidth0", true),
        image_height0: json_bool(params, "imageHeight0", true),
        path_empty_d: json_bool(params, "pathEmptyD", true),
        polyline_empty_points: json_bool(params, "polylineEmptyPoints", true),
        polygon_empty_points: json_bool(params, "polygonEmptyPoints", true),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StyleContentKind {
    Text,
    Cdata,
}

#[derive(Debug, Clone)]
struct InlineStylesParams {
    only_matched_once: bool,
    remove_matched_selectors: bool,
    use_mqs: Vec<String>,
}

#[derive(Debug, Clone)]
struct InlineSelectorEntry {
    style_node_id: usize,
    rule_index: usize,
    selector: String,
    specificity: [u8; 3],
    order: usize,
    matched_elements: Vec<usize>,
}

fn merge_styles(doc: &mut Document) {
    let mut first_style_id = None;
    let mut collected_styles = String::new();
    let mut content_kind = StyleContentKind::Text;

    for node_id in 1..doc.nodes.len() {
        if is_in_foreign_object_subtree(doc, node_id) {
            continue;
        }
        let Some(element) = node_element(doc, node_id) else {
            continue;
        };
        if element.name != "style" {
            continue;
        }
        if let Some(style_type) = attribute_value(element.attributes.as_slice(), "type")
            && !style_type.is_empty()
            && style_type != "text/css"
        {
            continue;
        }

        let (css, saw_cdata) = collect_style_css(doc, node_id);
        if saw_cdata {
            content_kind = StyleContentKind::Cdata;
        }
        if css.trim().is_empty() {
            detach_node(doc, node_id);
            continue;
        }

        let media = node_element(doc, node_id)
            .and_then(|element| attribute_value(element.attributes.as_slice(), "media"))
            .map(str::to_string);
        if let Some(media) = media {
            collected_styles.push_str(format!("@media {media}{{{css}}}").as_str());
            if let NodeKind::Element(element) = &mut doc.node_mut(node_id).kind {
                element.attributes.retain(|attribute| attribute.name != "media");
            }
        } else {
            collected_styles.push_str(css.as_str());
        }

        if let Some(first_id) = first_style_id {
            detach_node(doc, node_id);
            replace_children_with_style_content(doc, first_id, collected_styles.as_str(), content_kind);
        } else {
            first_style_id = Some(node_id);
        }
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "inlineStyles needs one orchestration pass across selectors, stylesheets, and cleanup"
)]
fn inline_styles(doc: &mut Document, params: Option<&Value>) {
    let params = inline_styles_params(params);
    let mut stylesheets = HashMap::<usize, Vec<CssRule>>::new();
    let mut content_kinds = HashMap::<usize, StyleContentKind>::new();
    let mut selectors = Vec::<InlineSelectorEntry>::new();
    let mut order = 0usize;

    for node_id in 1..doc.nodes.len() {
        if is_in_foreign_object_subtree(doc, node_id) {
            continue;
        }
        let Some(element) = node_element(doc, node_id) else {
            continue;
        };
        if element.name != "style" {
            continue;
        }
        if let Some(style_type) = attribute_value(element.attributes.as_slice(), "type")
            && !style_type.is_empty()
            && style_type != "text/css"
        {
            continue;
        }

        let media = attribute_value(element.attributes.as_slice(), "media")
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        if let Some(media) = &media
            && !params.use_mqs.iter().any(|allowed| allowed == media)
        {
            continue;
        }

        let (css, saw_cdata) = collect_style_css(doc, node_id);
        let mut rules = parse_css_rules(css.as_str());
        if let Some(media_query) = media {
            for rule in &mut rules {
                if rule.media_query.is_none() {
                    rule.media_query = Some(media_query.clone());
                }
            }
        }
        if rules.is_empty() {
            continue;
        }

        let content_kind = if saw_cdata {
            StyleContentKind::Cdata
        } else {
            StyleContentKind::Text
        };
        for (rule_index, rule) in rules.iter().enumerate() {
            if rule.media_query.as_deref().is_some_and(|query| {
                !params.use_mqs.iter().any(|allowed| allowed == query)
            }) {
                continue;
            }
            for selector in &rule.selectors {
                let Some(specificity) = selector_specificity(selector.as_str()) else {
                    continue;
                };
                selectors.push(InlineSelectorEntry {
                    style_node_id: node_id,
                    rule_index,
                    selector: selector.clone(),
                    specificity,
                    order,
                    matched_elements: Vec::new(),
                });
                order += 1;
            }
        }
        stylesheets.insert(node_id, rules);
        content_kinds.insert(node_id, content_kind);
    }

    selectors.sort_by(|left, right| {
        left.specificity
            .cmp(&right.specificity)
            .then(left.order.cmp(&right.order))
            .reverse()
    });

    for selector in &mut selectors {
        let matched_elements = matching_selector_nodes(doc, selector.selector.as_str());
        if matched_elements.is_empty() {
            continue;
        }
        selector.matched_elements.clone_from(&matched_elements);
        if !params.only_matched_once || matched_elements.len() == 1 {
            let declarations = stylesheets
                .get(&selector.style_node_id)
                .and_then(|rules| rules.get(selector.rule_index))
                .map(|rule| rule.declarations.clone())
                .unwrap_or_default();
            let remaining_selectors = collect_remaining_selectors(&stylesheets);
            for node_id in &matched_elements {
                inline_rule_declarations(
                    doc,
                    *node_id,
                    &declarations,
                    &remaining_selectors,
                );
            }
        }
        if params.remove_matched_selectors
            && let Some(rule) = stylesheets
                .get_mut(&selector.style_node_id)
                .and_then(|rules| rules.get_mut(selector.rule_index))
        {
            remove_first_selector(rule, selector.selector.as_str());
        }
    }

    if params.remove_matched_selectors {
        let remaining_selectors = collect_remaining_selectors(&stylesheets);
        for selector in &selectors {
            if selector.matched_elements.is_empty()
                || (params.only_matched_once && selector.matched_elements.len() > 1)
            {
                continue;
            }
            let selector_classes = extract_selector_classes(selector.selector.as_str());
            let selector_ids = extract_selector_ids(selector.selector.as_str());
            for node_id in &selector.matched_elements {
                cleanup_inlined_selector_attrs(
                    doc,
                    *node_id,
                    &selector_classes,
                    &selector_ids,
                    &remaining_selectors,
                );
            }
        }
    }

    for (style_node_id, rules) in stylesheets {
        let compact_css = serialize_css_rules(
            &rules
                .into_iter()
                .filter(|rule| !rule.selectors.is_empty() && !rule.declarations.is_empty())
                .collect::<Vec<_>>(),
        );
        if compact_css.is_empty() {
            detach_node(doc, style_node_id);
            continue;
        }
        let content_kind = content_kinds
            .get(&style_node_id)
            .copied()
            .unwrap_or(StyleContentKind::Text);
        let content_kind = if matches!(content_kind, StyleContentKind::Cdata)
            || compact_css.contains('<')
            || compact_css.contains('>')
        {
            StyleContentKind::Cdata
        } else {
            content_kind
        };
        replace_children_with_style_content(doc, style_node_id, compact_css.as_str(), content_kind);
    }
}

fn minify_styles(doc: &mut Document, _params: Option<&Value>) {
    let mut style_nodes = Vec::<(usize, StyleContentKind, String)>::new();
    let mut style_attr_nodes = Vec::<usize>::new();

    for node_id in 1..doc.nodes.len() {
        let Some(element) = node_element(doc, node_id) else {
            continue;
        };
        if element.name == "style" {
            let (css, saw_cdata) = collect_style_css(doc, node_id);
            style_nodes.push((
                node_id,
                if saw_cdata {
                    StyleContentKind::Cdata
                } else {
                    StyleContentKind::Text
                },
                css,
            ));
        } else if attribute_value(element.attributes.as_slice(), "style").is_some() {
            style_attr_nodes.push(node_id);
        }
    }

    for (node_id, content_kind, css) in style_nodes {
        let minified = minify_css_text(doc, css.as_str());
        if minified.is_empty() {
            detach_node(doc, node_id);
            continue;
        }
        let content_kind = if matches!(content_kind, StyleContentKind::Cdata)
            || minified.contains('<')
            || minified.contains('>')
        {
            StyleContentKind::Cdata
        } else {
            content_kind
        };
        replace_children_with_style_content(doc, node_id, minified.as_str(), content_kind);
    }

    for node_id in style_attr_nodes {
        let Some(element) = node_element(doc, node_id) else {
            continue;
        };
        let Some(style_value) = attribute_value(element.attributes.as_slice(), "style") else {
            continue;
        };
        let declarations = parse_style_declarations(style_value);
        let NodeKind::Element(element) = &mut doc.node_mut(node_id).kind else {
            continue;
        };
        let value = serialize_minified_style_declarations(&declarations);
        if value.is_empty() {
            element.attributes.retain(|attribute| attribute.name != "style");
        } else {
            set_or_push_attribute(
                &mut element.attributes,
                "style",
                value.as_str(),
                QuoteStyle::Double,
            );
        }
    }
}

fn inline_styles_params(params: Option<&Value>) -> InlineStylesParams {
    InlineStylesParams {
        only_matched_once: json_bool(params, "onlyMatchedOnce", true),
        remove_matched_selectors: json_bool(params, "removeMatchedSelectors", true),
        use_mqs: json_string_array(params, "useMqs", &["", "screen"]),
    }
}

fn json_string_array(params: Option<&Value>, name: &str, defaults: &[&str]) -> Vec<String> {
    params
        .and_then(|value| value.get(name))
        .and_then(Value::as_array)
        .map(|items| {
            items.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|items| !items.is_empty())
        .unwrap_or_else(|| defaults.iter().map(|item| (*item).to_string()).collect())
}

fn matching_selector_nodes(doc: &Document, selector: &str) -> Vec<usize> {
    doc.nodes
        .iter()
        .enumerate()
        .skip(1)
        .filter_map(|(node_id, node)| {
            matches!(node.kind, NodeKind::Element(_))
                .then_some(node_id)
                .filter(|node_id| selector_matches(doc, *node_id, selector))
        })
        .collect()
}

fn inline_rule_declarations(
    doc: &mut Document,
    node_id: usize,
    declarations: &[StyleDeclaration],
    remaining_selectors: &[String],
) {
    let NodeKind::Element(element) = &mut doc.node_mut(node_id).kind else {
        return;
    };

    let mut inline_styles = attribute_value(element.attributes.as_slice(), "style")
        .map(parse_style_declarations)
        .unwrap_or_default();
    let mut prepended = Vec::new();
    for declaration in declarations.iter().rev() {
        if is_presentation_attr(declaration.name.as_str())
            && !remaining_selectors.iter().any(|selector| {
                selector.contains(format!("[{}", declaration.name).as_str())
            })
        {
            element
                .attributes
                .retain(|attribute| attribute.name != declaration.name);
        }

        match inline_styles
            .iter()
            .position(|style_declaration| style_declaration.name == declaration.name)
        {
            Some(index) if !inline_styles[index].important && declaration.important => {
                inline_styles[index] = declaration.clone();
            }
            Some(_) => {}
            None => prepended.push(declaration.clone()),
        }
    }
    if !prepended.is_empty() {
        prepended.reverse();
        prepended.extend(inline_styles);
        inline_styles = prepended;
    }
    update_style_attribute(&mut element.attributes, &inline_styles);
}

fn collect_remaining_selectors(stylesheets: &HashMap<usize, Vec<CssRule>>) -> Vec<String> {
    stylesheets
        .values()
        .flat_map(|rules| rules.iter())
        .flat_map(|rule| rule.selectors.iter().cloned())
        .collect()
}

fn remove_first_selector(rule: &mut CssRule, selector: &str) {
    if let Some(index) = rule.selectors.iter().position(|item| item == selector) {
        rule.selectors.remove(index);
    }
}

fn extract_selector_classes(selector: &str) -> Vec<String> {
    extract_selector_tokens(selector, '.')
}

fn extract_selector_ids(selector: &str) -> Vec<String> {
    extract_selector_tokens(selector, '#')
}

fn extract_selector_tokens(selector: &str, marker: char) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut bracket_depth = 0usize;
    let bytes = selector.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'[' => bracket_depth += 1,
            b']' => bracket_depth = bracket_depth.saturating_sub(1),
            _ => {}
        }
        if bracket_depth == 0 && char::from(bytes[index]) == marker {
            index += 1;
            let start = index;
            while index < bytes.len() {
                let ch = char::from(bytes[index]);
                if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-') {
                    index += 1;
                } else {
                    break;
                }
            }
            if start < index {
                tokens.push(selector[start..index].to_string());
            }
            continue;
        }
        index += 1;
    }
    tokens
}

fn cleanup_inlined_selector_attrs(
    doc: &mut Document,
    node_id: usize,
    selector_classes: &[String],
    selector_ids: &[String],
    remaining_selectors: &[String],
) {
    let NodeKind::Element(element) = &mut doc.node_mut(node_id).kind else {
        return;
    };

    if !selector_classes.is_empty() {
        let mut class_list = attribute_value(element.attributes.as_slice(), "class")
            .map(|value| {
                value
                    .split_ascii_whitespace()
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        class_list.retain(|class_name| {
            !selector_classes.contains(class_name)
                || remaining_selectors.iter().any(|selector| {
                    selector.contains(format!(".{class_name}").as_str())
                        || selector.contains("[class")
                })
        });
        if class_list.is_empty() {
            element.attributes.retain(|attribute| attribute.name != "class");
        } else {
            set_or_push_attribute(
                &mut element.attributes,
                "class",
                class_list.join(" ").as_str(),
                QuoteStyle::Double,
            );
        }
    }

    if !selector_ids.is_empty() {
        let Some(id_value) = attribute_value(element.attributes.as_slice(), "id").map(str::to_string)
        else {
            return;
        };
        if selector_ids.contains(&id_value)
            && !remaining_selectors.iter().any(|selector| {
                selector.contains(format!("#{id_value}").as_str()) || selector.contains("[id")
            })
        {
            element.attributes.retain(|attribute| attribute.name != "id");
        }
    }
}

fn minify_css_text(doc: &Document, css: &str) -> String {
    let mut rules = parse_css_rules(css);
    if !element_has_scripts_in_document(doc) {
        let usage = collect_selector_usage(doc);
        for rule in &mut rules {
            rule.selectors
                .retain(|selector| selector_used(selector.as_str(), &usage));
        }
        rules.retain(|rule| !rule.selectors.is_empty());
    }
    if !rules.is_empty() {
        return serialize_css_rules(&rules);
    }
    strip_css_whitespace(css)
}

#[derive(Debug, Default)]
struct SelectorUsage {
    tags: HashSet<String>,
    ids: HashSet<String>,
    classes: HashSet<String>,
}

fn collect_selector_usage(doc: &Document) -> SelectorUsage {
    let mut usage = SelectorUsage::default();
    for node_id in 1..doc.nodes.len() {
        let Some(element) = node_element(doc, node_id) else {
            continue;
        };
        usage.tags.insert(element.name.clone());
        if let Some(id) = attribute_value(element.attributes.as_slice(), "id") {
            usage.ids.insert(id.to_string());
        }
        if let Some(class_names) = attribute_value(element.attributes.as_slice(), "class") {
            usage
                .classes
                .extend(class_names.split_ascii_whitespace().map(str::to_string));
        }
    }
    usage
}

fn selector_used(selector: &str, usage: &SelectorUsage) -> bool {
    let selector = selector.trim();
    if selector == "*" {
        return true;
    }
    if let Some(id) = selector.strip_prefix('#')
        && is_simple_selector(id)
    {
        return usage.ids.contains(id);
    }
    if let Some(class_name) = selector.strip_prefix('.')
        && is_simple_selector(class_name)
    {
        return usage.classes.contains(class_name);
    }
    if is_simple_selector(selector) {
        return usage.tags.contains(selector);
    }
    true
}

fn element_has_scripts_in_document(doc: &Document) -> bool {
    doc.nodes
        .iter()
        .enumerate()
        .skip(1)
        .any(|(node_id, _)| element_has_scripts(doc, node_id))
}

fn strip_css_whitespace(css: &str) -> String {
    let css = css
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    css.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn remove_non_inheritable_group_attrs(doc: &mut Document) {
    for node in &mut doc.nodes {
        let NodeKind::Element(element) = &mut node.kind else {
            continue;
        };
        if element.name != "g" {
            continue;
        }
        element.attributes.retain(|attribute| {
            !is_presentation_attr(attribute.name.as_str())
                || is_inheritable_attr(attribute.name.as_str())
                || is_preserved_group_presentation_attr(attribute.name.as_str())
        });
    }
}

fn remove_useless_stroke_and_fill(doc: &mut Document, params: Option<&Value>) {
    if document_has_style_or_scripts(doc) {
        return;
    }

    let options = PaintCleanupOptions::from_params(params);
    let root_children: Vec<_> = doc.children(doc.root_id()).collect();
    for child_id in root_children {
        traverse_paint_cleanup(doc, child_id, None, options);
    }
}

#[derive(Debug, Clone, Copy)]
struct PaintCleanupOptions {
    remove_stroke: bool,
    remove_fill: bool,
    remove_none: bool,
}

impl PaintCleanupOptions {
    fn from_params(params: Option<&Value>) -> Self {
        Self {
            remove_stroke: params
                .and_then(|value| value.get("stroke"))
                .and_then(Value::as_bool)
                .unwrap_or(true),
            remove_fill: params
                .and_then(|value| value.get("fill"))
                .and_then(Value::as_bool)
                .unwrap_or(true),
            remove_none: params
                .and_then(|value| value.get("removeNone"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
        }
    }
}

#[derive(Debug, Clone, Default)]
struct PaintStyle {
    stroke: Option<String>,
    stroke_opacity: Option<String>,
    stroke_width: Option<String>,
    marker_end: Option<String>,
    fill: Option<String>,
    fill_opacity: Option<String>,
}

fn traverse_paint_cleanup(
    doc: &mut Document,
    node_id: usize,
    parent_style: Option<&PaintStyle>,
    options: PaintCleanupOptions,
) {
    let current_style = compute_paint_style(doc, node_id, parent_style);
    let Some(element_name) = node_element_name(doc, node_id).map(str::to_string) else {
        return;
    };

    if node_has_id(doc, node_id) {
        return;
    }

    if is_shape_element(element_name.as_str()) {
        cleanup_shape_paint(doc, node_id, &current_style, parent_style, options);
        if doc.node(node_id).parent.is_none() {
            return;
        }
    }

    let child_ids: Vec<_> = doc.children(node_id).collect();
    for child_id in child_ids {
        traverse_paint_cleanup(doc, child_id, Some(&current_style), options);
    }
}

fn cleanup_shape_paint(
    doc: &mut Document,
    node_id: usize,
    current_style: &PaintStyle,
    parent_style: Option<&PaintStyle>,
    options: PaintCleanupOptions,
) {
    let parent_stroke = parent_style.and_then(|style| style.stroke.as_deref());

    if options.remove_stroke
        && (current_style.stroke.is_none()
            || current_style.stroke.as_deref() == Some("none")
            || current_style.stroke_opacity.as_deref() == Some("0")
            || current_style.stroke_width.as_deref() == Some("0"))
        && (current_style.stroke_width.as_deref() == Some("0")
            || current_style.marker_end.is_none())
    {
        let NodeKind::Element(element) = &mut doc.node_mut(node_id).kind else {
            return;
        };
        element
            .attributes
            .retain(|attribute| !attribute.name.starts_with("stroke"));
        if parent_stroke.is_some_and(|stroke| stroke != "none") {
            element.attributes.push(Attribute {
                name: "stroke".to_string(),
                value: "none".to_string(),
                quote: QuoteStyle::Double,
            });
        }
    }

    if options.remove_fill
        && (current_style.fill.as_deref() == Some("none")
            || current_style.fill_opacity.as_deref() == Some("0"))
    {
        let NodeKind::Element(element) = &mut doc.node_mut(node_id).kind else {
            return;
        };
        element
            .attributes
            .retain(|attribute| !attribute.name.starts_with("fill-"));
        if current_style.fill.as_deref() != Some("none") {
            set_or_push_attribute(&mut element.attributes, "fill", "none", QuoteStyle::Double);
        }
    }

    if options.remove_none {
        let stroke_none = current_style.stroke.is_none()
            || attribute_value(
                doc.node(node_id).kind.element_attributes().as_slice(),
                "stroke",
            ) == Some("none");
        let fill_none = current_style.fill.as_deref() == Some("none")
            || attribute_value(
                doc.node(node_id).kind.element_attributes().as_slice(),
                "fill",
            ) == Some("none");
        if stroke_none && fill_none {
            detach_node(doc, node_id);
        }
    }
}

fn compute_paint_style(
    doc: &Document,
    node_id: usize,
    parent_style: Option<&PaintStyle>,
) -> PaintStyle {
    let Some(element) = node_element(doc, node_id) else {
        return parent_style.cloned().unwrap_or_default();
    };
    let inline_style = attribute_value(element.attributes.as_slice(), "style")
        .map(parse_style_declarations)
        .unwrap_or_default();

    let mut style = PaintStyle {
        stroke: parent_style.and_then(|parent| parent.stroke.clone()),
        stroke_opacity: parent_style.and_then(|parent| parent.stroke_opacity.clone()),
        stroke_width: parent_style.and_then(|parent| parent.stroke_width.clone()),
        marker_end: parent_style.and_then(|parent| parent.marker_end.clone()),
        fill: parent_style.and_then(|parent| parent.fill.clone()),
        fill_opacity: parent_style.and_then(|parent| parent.fill_opacity.clone()),
    };

    apply_paint_style_value(
        &mut style,
        "stroke",
        attribute_value(element.attributes.as_slice(), "stroke"),
    );
    apply_paint_style_value(
        &mut style,
        "stroke-opacity",
        attribute_value(element.attributes.as_slice(), "stroke-opacity"),
    );
    apply_paint_style_value(
        &mut style,
        "stroke-width",
        attribute_value(element.attributes.as_slice(), "stroke-width"),
    );
    apply_paint_style_value(
        &mut style,
        "marker-end",
        attribute_value(element.attributes.as_slice(), "marker-end"),
    );
    apply_paint_style_value(
        &mut style,
        "fill",
        attribute_value(element.attributes.as_slice(), "fill"),
    );
    apply_paint_style_value(
        &mut style,
        "fill-opacity",
        attribute_value(element.attributes.as_slice(), "fill-opacity"),
    );

    for declaration in &inline_style {
        apply_paint_style_value(
            &mut style,
            declaration.name.as_str(),
            Some(declaration.value.as_str()),
        );
    }

    style
}

fn apply_paint_style_value(style: &mut PaintStyle, name: &str, value: Option<&str>) {
    let Some(value) = value else {
        return;
    };
    match name {
        "stroke" => style.stroke = Some(value.to_string()),
        "stroke-opacity" => style.stroke_opacity = Some(value.to_string()),
        "stroke-width" => style.stroke_width = Some(value.to_string()),
        "marker-end" => style.marker_end = Some(value.to_string()),
        "fill" => style.fill = Some(value.to_string()),
        "fill-opacity" => style.fill_opacity = Some(value.to_string()),
        _ => {}
    }
}

fn move_group_attrs_to_elems(doc: &mut Document) {
    let target_ids: Vec<_> = doc
        .nodes
        .iter()
        .enumerate()
        .skip(1)
        .filter_map(|(id, node)| {
            let NodeKind::Element(element) = &node.kind else {
                return None;
            };
            if element.name != "g" || doc.node(id).first_child.is_none() {
                return None;
            }

            let transform = attribute_named(element.attributes.as_slice(), "transform")?;

            if element.attributes.iter().any(|attribute| {
                is_reference_property(attribute.name.as_str())
                    && includes_url_reference(attribute.value.as_str())
            }) {
                return None;
            }

            if !doc.children(id).all(|child_id| {
                let NodeKind::Element(child) = &doc.node(child_id).kind else {
                    return false;
                };
                is_group_transform_target(child.name.as_str())
                    && attribute_value(child.attributes.as_slice(), "id").is_none()
            }) {
                return None;
            }

            let _ = transform;
            Some(id)
        })
        .collect();

    for group_id in target_ids {
        let transform = match &doc.node(group_id).kind {
            NodeKind::Element(group) => {
                let Some(transform) = attribute_named(group.attributes.as_slice(), "transform")
                else {
                    continue;
                };
                transform.clone()
            }
            _ => continue,
        };
        let child_ids: Vec<_> = doc.children(group_id).collect();
        for child_id in child_ids {
            let NodeKind::Element(child) = &mut doc.node_mut(child_id).kind else {
                continue;
            };
            if let Some(existing) =
                attribute_named_mut(child.attributes.as_mut_slice(), "transform")
            {
                existing.value = format!("{} {}", transform.value, existing.value);
            } else {
                child.attributes.push(transform.clone());
            }
        }

        let NodeKind::Element(group) = &mut doc.node_mut(group_id).kind else {
            continue;
        };
        group
            .attributes
            .retain(|attribute| attribute.name != "transform");
    }
}

fn move_elems_attrs_to_group(doc: &mut Document) {
    if doc
        .nodes
        .iter()
        .any(|node| matches!(&node.kind, NodeKind::Element(element) if element.name == "style"))
    {
        return;
    }

    let target_ids: Vec<_> = doc
        .nodes
        .iter()
        .enumerate()
        .skip(1)
        .rev()
        .filter_map(|(id, node)| match &node.kind {
            NodeKind::Element(element) if element.name == "g" && doc.children(id).count() > 1 => {
                Some(id)
            }
            _ => None,
        })
        .collect();

    for group_id in target_ids {
        let child_ids: Vec<_> = doc.children(group_id).collect();
        let mut common_attributes: Vec<Attribute> = Vec::new();
        let mut initialized = false;
        let mut every_child_is_path = true;

        for child_id in &child_ids {
            let NodeKind::Element(child) = &doc.node(*child_id).kind else {
                continue;
            };

            if !is_path_element(child.name.as_str()) {
                every_child_is_path = false;
            }

            if initialized {
                common_attributes.retain(|attribute| {
                    attribute_value(child.attributes.as_slice(), attribute.name.as_str())
                        == Some(attribute.value.as_str())
                });
            } else {
                initialized = true;
                common_attributes = child
                    .attributes
                    .iter()
                    .filter(|attribute| is_inheritable_attr(attribute.name.as_str()))
                    .cloned()
                    .collect();
            }
        }

        if common_attributes.is_empty() {
            continue;
        }

        let group_attributes = match &doc.node(group_id).kind {
            NodeKind::Element(group) => group.attributes.clone(),
            _ => continue,
        };
        if attribute_value(group_attributes.as_slice(), "filter").is_some()
            || attribute_value(group_attributes.as_slice(), "clip-path").is_some()
            || attribute_value(group_attributes.as_slice(), "mask").is_some()
            || every_child_is_path
        {
            common_attributes.retain(|attribute| attribute.name != "transform");
        }

        if common_attributes.is_empty() {
            continue;
        }

        {
            let NodeKind::Element(group) = &mut doc.node_mut(group_id).kind else {
                continue;
            };
            for attribute in &common_attributes {
                if attribute.name == "transform" {
                    if let Some(existing) =
                        attribute_named_mut(group.attributes.as_mut_slice(), "transform")
                    {
                        existing.value = format!("{} {}", existing.value, attribute.value);
                    } else {
                        group.attributes.push(attribute.clone());
                    }
                } else if let Some(existing) =
                    attribute_named_mut(group.attributes.as_mut_slice(), attribute.name.as_str())
                {
                    existing.value.clone_from(&attribute.value);
                    existing.quote = attribute.quote;
                } else {
                    group.attributes.push(attribute.clone());
                }
            }
        }

        let names: HashSet<_> = common_attributes
            .iter()
            .map(|attribute| attribute.name.clone())
            .collect();
        for child_id in child_ids {
            let NodeKind::Element(child) = &mut doc.node_mut(child_id).kind else {
                continue;
            };
            child
                .attributes
                .retain(|attribute| !names.contains(&attribute.name));
        }
    }
}

fn collapse_groups(doc: &mut Document) {
    let stylesheet = collect_filter_stylesheet(doc);
    let target_ids: Vec<_> = doc
        .nodes
        .iter()
        .enumerate()
        .skip(1)
        .rev()
        .filter_map(|(id, node)| match &node.kind {
            NodeKind::Element(element) if element.name == "g" => Some(id),
            _ => None,
        })
        .collect();

    for group_id in target_ids {
        let Some(parent_id) = doc.node(group_id).parent else {
            continue;
        };
        match &doc.node(parent_id).kind {
            NodeKind::Document => continue,
            NodeKind::Element(parent) if parent.name == "switch" => continue,
            _ => (),
        }

        if doc.node(group_id).first_child.is_none() {
            continue;
        }

        collapse_single_child_group(doc, group_id, &stylesheet);

        let should_collapse = match &doc.node(group_id).kind {
            NodeKind::Element(group) => group.attributes.is_empty(),
            _ => false,
        };

        if should_collapse && !group_has_animation_children(doc, group_id) {
            replace_node_with_children(doc, group_id);
        }
    }
}

fn collapse_single_child_group(doc: &mut Document, group_id: usize, stylesheet: &FilterStylesheet) {
    let child_ids: Vec<_> = doc.children(group_id).collect();
    if child_ids.len() != 1 {
        return;
    }

    let child_id = child_ids[0];
    let group_attributes = match &doc.node(group_id).kind {
        NodeKind::Element(group) if !group.attributes.is_empty() => group.attributes.clone(),
        _ => return,
    };

    let can_merge = {
        let NodeKind::Element(child) = &doc.node(child_id).kind else {
            return;
        };
        if attribute_value(child.attributes.as_slice(), "id").is_some() {
            return;
        }
        if element_has_filter(group_attributes.as_slice(), stylesheet) {
            return;
        }
        if attribute_value(group_attributes.as_slice(), "class").is_some()
            && attribute_value(child.attributes.as_slice(), "class").is_some()
        {
            return;
        }
        if (attribute_value(group_attributes.as_slice(), "clip-path").is_some()
            || attribute_value(group_attributes.as_slice(), "mask").is_some())
            && !(child.name == "g"
                && attribute_value(group_attributes.as_slice(), "transform").is_none()
                && attribute_value(child.attributes.as_slice(), "transform").is_none())
        {
            return;
        }
        if group_attributes
            .iter()
            .any(|attribute| has_animated_attr(doc, child_id, attribute.name.as_str()))
        {
            return;
        }

        let mut merged_attributes = child.attributes.clone();
        for attribute in &group_attributes {
            if let Some(existing) =
                attribute_named_mut(merged_attributes.as_mut_slice(), attribute.name.as_str())
            {
                if attribute.name == "transform" {
                    existing.value = format!("{} {}", attribute.value, existing.value);
                } else if existing.value == "inherit" {
                    existing.value.clone_from(&attribute.value);
                    existing.quote = attribute.quote;
                } else if !is_inheritable_attr(attribute.name.as_str())
                    && existing.value != attribute.value
                {
                    return;
                }
            } else {
                merged_attributes.push(attribute.clone());
            }
        }
        merged_attributes
    };

    let NodeKind::Element(group) = &mut doc.node_mut(group_id).kind else {
        return;
    };
    group.attributes.clear();

    let NodeKind::Element(child) = &mut doc.node_mut(child_id).kind else {
        return;
    };
    child.attributes = can_merge;
}

fn remove_empty_containers(doc: &mut Document) {
    let stylesheet = collect_filter_stylesheet(doc);
    let targets: Vec<_> = doc
        .nodes
        .iter()
        .enumerate()
        .skip(1)
        .filter_map(|(id, node)| {
            let NodeKind::Element(element) = &node.kind else {
                return None;
            };
            if should_remove_empty_container(doc, id, element.name.as_str(), &stylesheet) {
                return Some(id);
            }
            None
        })
        .collect();

    let mut removed_ids = HashSet::new();
    for target_id in targets {
        if let NodeKind::Element(element) = &doc.node(target_id).kind
            && let Some(id) = attribute_value(element.attributes.as_slice(), "id")
        {
            removed_ids.insert(id.to_string());
        }
        detach_node(doc, target_id);
    }

    if removed_ids.is_empty() {
        return;
    }

    let uses_to_remove: Vec<_> = doc
        .nodes
        .iter()
        .enumerate()
        .skip(1)
        .filter_map(|(id, node)| match &node.kind {
            NodeKind::Element(element)
                if element.name == "use"
                    && element.attributes.iter().any(|attribute| {
                        value_references_any_id(&attribute.value, &removed_ids)
                    }) =>
            {
                Some(id)
            }
            _ => None,
        })
        .collect();

    for use_id in uses_to_remove {
        detach_node(doc, use_id);
    }
}

fn remove_desc(doc: &mut Document, params: Option<&Value>) {
    let remove_any = params
        .and_then(|value| value.get("removeAny"))
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let mut ids = Vec::new();
    for (id, node) in doc.nodes.iter().enumerate().skip(1) {
        let NodeKind::Element(element) = &node.kind else {
            continue;
        };
        if element.name != "desc" {
            continue;
        }
        if remove_any || desc_is_removable(doc, id) {
            ids.push(id);
        }
    }
    for id in ids {
        detach_node(doc, id);
    }
}

fn desc_is_removable(doc: &Document, id: usize) -> bool {
    let mut text = String::new();
    for child in doc.children(id) {
        match &doc.node(child).kind {
            NodeKind::Text(value) => text.push_str(value),
            _ => return false,
        }
    }
    let normalized = text.trim_start();
    normalized.is_empty()
        || normalized.starts_with("Created with")
        || normalized.starts_with("Created using")
}

fn remove_dimensions(doc: &mut Document) {
    let Some(root_id) = find_root_svg(doc) else {
        return;
    };
    let NodeKind::Element(element) = &mut doc.node_mut(root_id).kind else {
        return;
    };

    let mut width = None;
    let mut height = None;
    let mut view_box_exists = false;
    element
        .attributes
        .retain(|attribute| match attribute.name.as_str() {
            "width" => {
                width = Some(attribute.value.clone());
                false
            }
            "height" => {
                height = Some(attribute.value.clone());
                false
            }
            "viewBox" => {
                view_box_exists = true;
                true
            }
            _ => true,
        });

    if !view_box_exists && let (Some(width), Some(height)) = (width, height) {
        element.attributes.push(Attribute {
            name: "viewBox".to_string(),
            value: format!("0 0 {width} {height}"),
            quote: QuoteStyle::Double,
        });
    }
}

fn sort_attrs(doc: &mut Document, params: Option<&Value>) {
    let order = params
        .and_then(|value| value.get("order"))
        .and_then(Value::as_array)
        .map_or_else(default_sort_attr_order, |array| {
            array
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        });
    let xmlns_order = params
        .and_then(|value| value.get("xmlnsOrder"))
        .and_then(Value::as_str)
        .unwrap_or("front");

    for node in &mut doc.nodes {
        let NodeKind::Element(element) = &mut node.kind else {
            continue;
        };
        element
            .attributes
            .sort_by(|left, right| compare_attrs(&left.name, &right.name, &order, xmlns_order));
    }
}

fn sort_defs_children(doc: &mut Document) {
    let defs_ids: Vec<_> = doc
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(id, node)| match &node.kind {
            NodeKind::Element(element) if element.name == "defs" => Some(id),
            _ => None,
        })
        .collect();

    for defs_id in defs_ids {
        let mut children: Vec<_> = doc.children(defs_id).collect();
        let mut frequencies = std::collections::BTreeMap::<String, usize>::new();
        for child_id in &children {
            if let NodeKind::Element(element) = &doc.node(*child_id).kind {
                *frequencies.entry(element.name.clone()).or_default() += 1;
            }
        }

        children.sort_by(|left, right| {
            let left_node = doc.node(*left);
            let right_node = doc.node(*right);
            let (NodeKind::Element(left_element), NodeKind::Element(right_element)) =
                (&left_node.kind, &right_node.kind)
            else {
                return std::cmp::Ordering::Equal;
            };

            let left_frequency = frequencies.get(&left_element.name).copied().unwrap_or(0);
            let right_frequency = frequencies.get(&right_element.name).copied().unwrap_or(0);
            match right_frequency.cmp(&left_frequency) {
                std::cmp::Ordering::Equal => {}
                ordering => return ordering,
            }
            match right_element.name.len().cmp(&left_element.name.len()) {
                std::cmp::Ordering::Equal => {}
                ordering => return ordering,
            }
            right_element.name.cmp(&left_element.name)
        });

        doc.reorder_children(defs_id, &children);
    }
}

fn remove_deprecated_attrs(doc: &mut Document, params: Option<&Value>) {
    let remove_unsafe = params
        .and_then(|value| value.get("removeUnsafe"))
        .and_then(Value::as_bool)
        .unwrap_or(false);

    for node in &mut doc.nodes {
        let NodeKind::Element(element) = &mut node.kind else {
            continue;
        };
        match element.name.as_str() {
            "svg" => {
                element.attributes.retain(|attribute| {
                    if attribute.name == "version" {
                        return false;
                    }
                    if remove_unsafe && attribute.name == "enable-background" {
                        return false;
                    }
                    true
                });
            }
            "view" => {
                if remove_unsafe {
                    element
                        .attributes
                        .retain(|attribute| attribute.name != "viewTarget");
                }
            }
            "text" => {
                let has_lang = element
                    .attributes
                    .iter()
                    .any(|attribute| attribute.name == "lang");
                element.attributes.retain(|attribute| {
                    if attribute.name == "xml:lang" {
                        return !(has_lang || remove_unsafe);
                    }
                    true
                });
            }
            _ => {}
        }
    }
}

fn remove_unused_ns(doc: &mut Document) {
    let Some(root_id) = find_root_svg(doc) else {
        return;
    };

    let mut unused = doc
        .node(root_id)
        .kind
        .element_attributes()
        .into_iter()
        .filter_map(|attribute| attribute.name.strip_prefix("xmlns:").map(ToOwned::to_owned))
        .collect::<std::collections::BTreeSet<_>>();

    if unused.is_empty() {
        return;
    }

    for (id, node) in doc.nodes.iter().enumerate().skip(1) {
        if id != root_id && !node_is_attached_to_root(doc, root_id, id) {
            continue;
        }
        let NodeKind::Element(element) = &node.kind else {
            continue;
        };
        if id != root_id
            && let Some((prefix, _)) = element.name.split_once(':')
        {
            unused.remove(prefix);
        }
        for attribute in &element.attributes {
            if let Some((prefix, _)) = attribute.name.split_once(':') {
                unused.remove(prefix);
            }
        }
    }

    let NodeKind::Element(root) = &mut doc.node_mut(root_id).kind else {
        return;
    };
    root.attributes.retain(|attribute| {
        attribute
            .name
            .strip_prefix("xmlns:")
            .is_none_or(|prefix| !unused.contains(prefix))
    });
}

fn node_is_attached_to_root(doc: &Document, root_id: usize, node_id: usize) -> bool {
    let mut current = Some(node_id);
    while let Some(id) = current {
        if id == root_id {
            return true;
        }
        current = doc.node(id).parent;
    }
    false
}

fn remove_editors_ns_data(doc: &mut Document, params: Option<&Value>) {
    let mut namespaces = editor_namespaces();
    if let Some(additional) = params
        .and_then(|value| value.get("additionalNamespaces"))
        .and_then(Value::as_array)
    {
        namespaces.extend(
            additional
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned),
        );
    }

    let Some(root_id) = find_root_svg(doc) else {
        return;
    };
    let NodeKind::Element(root) = &mut doc.node_mut(root_id).kind else {
        return;
    };

    let mut prefixes = Vec::new();
    root.attributes.retain(|attribute| {
        if let Some(prefix) = attribute.name.strip_prefix("xmlns:")
            && namespaces
                .iter()
                .any(|namespace| namespace == &attribute.value)
        {
            prefixes.push(prefix.to_string());
            return false;
        }
        true
    });

    if prefixes.is_empty() {
        return;
    }

    let mut remove_nodes = Vec::new();
    for (id, node) in doc.nodes.iter_mut().enumerate().skip(1) {
        let NodeKind::Element(element) = &mut node.kind else {
            continue;
        };
        element.attributes.retain(|attribute| {
            attribute
                .name
                .split_once(':')
                .is_none_or(|(prefix, _)| !prefixes.iter().any(|candidate| candidate == prefix))
        });

        if element
            .name
            .split_once(':')
            .is_some_and(|(prefix, _)| prefixes.iter().any(|candidate| candidate == prefix))
        {
            remove_nodes.push(id);
        }
    }

    for id in remove_nodes {
        detach_node(doc, id);
    }
}

fn remove_empty_attrs(doc: &mut Document) {
    for node in &mut doc.nodes {
        let NodeKind::Element(element) = &mut node.kind else {
            continue;
        };
        element.attributes.retain(|attribute| {
            if attribute.value.is_empty() {
                matches!(
                    attribute.name.as_str(),
                    "requiredFeatures" | "requiredExtensions" | "systemLanguage"
                )
            } else {
                true
            }
        });
    }
}

fn remove_empty_text(doc: &mut Document, params: Option<&Value>) {
    let remove_text = params
        .and_then(|value| value.get("text"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let remove_tspan = params
        .and_then(|value| value.get("tspan"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let remove_tref = params
        .and_then(|value| value.get("tref"))
        .and_then(Value::as_bool)
        .unwrap_or(true);

    let mut ids = Vec::new();
    for (id, node) in doc.nodes.iter().enumerate().skip(1) {
        let NodeKind::Element(element) = &node.kind else {
            continue;
        };
        let has_children = doc.node(id).first_child.is_some();
        let should_remove = match element.name.as_str() {
            "text" => remove_text && !has_children,
            "tspan" => remove_tspan && !has_children,
            "tref" => {
                remove_tref
                    && !element
                        .attributes
                        .iter()
                        .any(|attr| attr.name == "xlink:href")
            }
            _ => false,
        };
        if should_remove {
            ids.push(id);
        }
    }
    for id in ids {
        detach_node(doc, id);
    }
}

fn remove_xmlns(doc: &mut Document) {
    let Some(root_id) = find_root_svg(doc) else {
        return;
    };
    let NodeKind::Element(element) = &mut doc.node_mut(root_id).kind else {
        return;
    };
    element
        .attributes
        .retain(|attribute| attribute.name.as_str() != "xmlns");
}

fn should_remove_empty_container(
    doc: &Document,
    node_id: usize,
    name: &str,
    stylesheet: &FilterStylesheet,
) -> bool {
    if name == "svg" || !is_container_element(name) || doc.node(node_id).first_child.is_some() {
        return false;
    }

    let NodeKind::Element(element) = &doc.node(node_id).kind else {
        return false;
    };

    if name == "pattern" && !element.attributes.is_empty() {
        return false;
    }

    if name == "mask" && attribute_value(element.attributes.as_slice(), "id").is_some() {
        return false;
    }

    if doc
        .node(node_id)
        .parent
        .and_then(|parent_id| match &doc.node(parent_id).kind {
            NodeKind::Element(parent) => Some(parent.name.as_str()),
            _ => None,
        })
        == Some("switch")
    {
        return false;
    }

    if name == "g" && element_has_filter(element.attributes.as_slice(), stylesheet) {
        return false;
    }

    true
}

fn is_group_transform_target(name: &str) -> bool {
    matches!(name, "glyph" | "missing-glyph" | "path" | "g" | "text")
}

fn is_path_element(name: &str) -> bool {
    matches!(name, "glyph" | "missing-glyph" | "path")
}

fn element_has_filter(attributes: &[Attribute], stylesheet: &FilterStylesheet) -> bool {
    if attribute_value(attributes, "filter").is_some() {
        return true;
    }

    if attribute_value(attributes, "style").is_some_and(style_declares_filter) {
        return true;
    }

    stylesheet.matches(attributes)
}

#[derive(Debug, Default)]
struct FilterStylesheet {
    has_universal_filter: bool,
    has_g_filter: bool,
    id_filters: HashSet<String>,
    class_filters: HashSet<String>,
}

impl FilterStylesheet {
    fn matches(&self, attributes: &[Attribute]) -> bool {
        if self.has_universal_filter || self.has_g_filter {
            return true;
        }

        if attribute_value(attributes, "id").is_some_and(|id| self.id_filters.contains(id)) {
            return true;
        }

        attribute_value(attributes, "class").is_some_and(|classes| {
            classes
                .split_whitespace()
                .any(|class_name| self.class_filters.contains(class_name))
        })
    }
}

fn collect_filter_stylesheet(doc: &Document) -> FilterStylesheet {
    let mut stylesheet = FilterStylesheet::default();
    for (node_id, node) in doc.nodes.iter().enumerate() {
        let NodeKind::Element(element) = &node.kind else {
            continue;
        };
        if element.name != "style" {
            continue;
        }
        for child_id in doc.children(node_id) {
            match &doc.node(child_id).kind {
                NodeKind::Text(text) | NodeKind::Cdata(text) => {
                    ingest_filter_rules(text, &mut stylesheet);
                }
                _ => {}
            }
        }
    }
    stylesheet
}

fn ingest_filter_rules(css: &str, stylesheet: &mut FilterStylesheet) {
    for rule in css.split('}') {
        let Some((selectors, declarations)) = rule.split_once('{') else {
            continue;
        };
        if !declarations_have_filter(declarations) {
            continue;
        }
        for selector in selectors.split(',').map(str::trim) {
            if selector.is_empty() {
                continue;
            }
            match selector {
                "*" => stylesheet.has_universal_filter = true,
                "g" => stylesheet.has_g_filter = true,
                _ => {
                    if let Some(id) = selector.strip_prefix('#')
                        && is_simple_selector(id)
                    {
                        stylesheet.id_filters.insert(id.to_string());
                    }
                    if let Some(class_name) = selector.strip_prefix('.')
                        && is_simple_selector(class_name)
                    {
                        stylesheet.class_filters.insert(class_name.to_string());
                    }
                    if let Some((element_name, class_name)) = selector.split_once('.')
                        && element_name == "g"
                        && is_simple_selector(class_name)
                    {
                        stylesheet.class_filters.insert(class_name.to_string());
                    }
                    if let Some((element_name, id)) = selector.split_once('#')
                        && element_name == "g"
                        && is_simple_selector(id)
                    {
                        stylesheet.id_filters.insert(id.to_string());
                    }
                }
            }
        }
    }
}

fn declarations_have_filter(declarations: &str) -> bool {
    declarations
        .split(';')
        .filter_map(|declaration| declaration.split_once(':'))
        .any(|(name, _)| name.trim() == "filter")
}

fn style_declares_filter(style: &str) -> bool {
    declarations_have_filter(style)
}

fn is_simple_selector(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|char| char.is_ascii_alphanumeric() || matches!(char, '_' | '-' | ':'))
}

fn attribute_value<'a>(attributes: &'a [Attribute], name: &str) -> Option<&'a str> {
    attributes
        .iter()
        .find(|attribute| attribute.name == name)
        .map(|attribute| attribute.value.as_str())
}

fn attribute_named<'a>(attributes: &'a [Attribute], name: &str) -> Option<&'a Attribute> {
    attributes.iter().find(|attribute| attribute.name == name)
}

fn attribute_named_mut<'a>(
    attributes: &'a mut [Attribute],
    name: &str,
) -> Option<&'a mut Attribute> {
    attributes
        .iter_mut()
        .find(|attribute| attribute.name == name)
}

fn value_references_any_id(value: &str, ids: &HashSet<String>) -> bool {
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'#' {
            index += 1;
            continue;
        }
        index += 1;
        let start = index;
        while index < bytes.len() {
            let char = char::from(bytes[index]);
            if char.is_ascii_alphanumeric() || matches!(char, '_' | '-' | '.' | ':') {
                index += 1;
            } else {
                break;
            }
        }
        if start < index && ids.contains(&value[start..index]) {
            return true;
        }
    }
    false
}

fn includes_url_reference(value: &str) -> bool {
    value.contains("url(") && value.contains('#')
}

fn node_element(doc: &Document, node_id: usize) -> Option<&crate::ast::Element> {
    match &doc.node(node_id).kind {
        NodeKind::Element(element) => Some(element),
        _ => None,
    }
}

fn node_element_name(doc: &Document, node_id: usize) -> Option<&str> {
    node_element(doc, node_id).map(|element| element.name.as_str())
}

fn node_has_id(doc: &Document, node_id: usize) -> bool {
    node_element(doc, node_id)
        .and_then(|element| attribute_value(element.attributes.as_slice(), "id"))
        .is_some()
}

fn set_or_push_attribute(
    attributes: &mut Vec<Attribute>,
    name: &str,
    value: &str,
    quote: QuoteStyle,
) {
    if let Some(attribute) = attribute_named_mut(attributes.as_mut_slice(), name) {
        attribute.value.clear();
        attribute.value.push_str(value);
        attribute.quote = quote;
    } else {
        attributes.push(Attribute {
            name: name.to_string(),
            value: value.to_string(),
            quote,
        });
    }
}

fn document_has_style_or_scripts(doc: &Document) -> bool {
    doc.nodes.iter().enumerate().skip(1).any(|(node_id, node)| {
        let NodeKind::Element(element) = &node.kind else {
            return false;
        };
        element.name == "style" || element_has_scripts(doc, node_id)
    })
}

fn element_has_scripts(doc: &Document, node_id: usize) -> bool {
    let Some(element) = node_element(doc, node_id) else {
        return false;
    };

    if element.name == "script" && doc.node(node_id).first_child.is_some() {
        return true;
    }

    if element.name == "a"
        && element.attributes.iter().any(|attribute| {
            (attribute.name == "href" || attribute.name.ends_with(":href"))
                && attribute.value.trim_start().starts_with("javascript:")
        })
    {
        return true;
    }

    element
        .attributes
        .iter()
        .any(|attribute| attribute.name.starts_with("on"))
}

fn is_shape_element(name: &str) -> bool {
    matches!(
        name,
        "circle" | "ellipse" | "line" | "path" | "polygon" | "polyline" | "rect"
    )
}

fn is_inheritable_attr(name: &str) -> bool {
    matches!(
        name,
        "clip-rule"
            | "color-interpolation-filters"
            | "color-interpolation"
            | "color-profile"
            | "color-rendering"
            | "color"
            | "cursor"
            | "direction"
            | "dominant-baseline"
            | "fill-opacity"
            | "fill-rule"
            | "fill"
            | "font-family"
            | "font-size-adjust"
            | "font-size"
            | "font-stretch"
            | "font-style"
            | "font-variant"
            | "font-weight"
            | "font"
            | "glyph-orientation-horizontal"
            | "glyph-orientation-vertical"
            | "image-rendering"
            | "letter-spacing"
            | "marker-end"
            | "marker-mid"
            | "marker-start"
            | "marker"
            | "paint-order"
            | "pointer-events"
            | "shape-rendering"
            | "stroke-dasharray"
            | "stroke-dashoffset"
            | "stroke-linecap"
            | "stroke-linejoin"
            | "stroke-miterlimit"
            | "stroke-opacity"
            | "stroke-width"
            | "stroke"
            | "text-anchor"
            | "text-rendering"
            | "transform"
            | "visibility"
            | "word-spacing"
            | "writing-mode"
    )
}

fn is_presentation_attr(name: &str) -> bool {
    matches!(
        name,
        "alignment-baseline"
            | "baseline-shift"
            | "clip-path"
            | "clip-rule"
            | "clip"
            | "color-interpolation-filters"
            | "color-interpolation"
            | "color-profile"
            | "color-rendering"
            | "color"
            | "cursor"
            | "direction"
            | "display"
            | "dominant-baseline"
            | "enable-background"
            | "fill-opacity"
            | "fill-rule"
            | "fill"
            | "filter"
            | "flood-color"
            | "flood-opacity"
            | "font-family"
            | "font-size-adjust"
            | "font-size"
            | "font-stretch"
            | "font-style"
            | "font-variant"
            | "font-weight"
            | "glyph-orientation-horizontal"
            | "glyph-orientation-vertical"
            | "image-rendering"
            | "letter-spacing"
            | "lighting-color"
            | "marker-end"
            | "marker-mid"
            | "marker-start"
            | "mask"
            | "opacity"
            | "overflow"
            | "paint-order"
            | "pointer-events"
            | "shape-rendering"
            | "stop-color"
            | "stop-opacity"
            | "stroke-dasharray"
            | "stroke-dashoffset"
            | "stroke-linecap"
            | "stroke-linejoin"
            | "stroke-miterlimit"
            | "stroke-opacity"
            | "stroke-width"
            | "stroke"
            | "text-anchor"
            | "text-decoration"
            | "text-overflow"
            | "text-rendering"
            | "transform-origin"
            | "transform"
            | "unicode-bidi"
            | "vector-effect"
            | "visibility"
            | "word-spacing"
            | "writing-mode"
    )
}

fn is_core_attr(name: &str) -> bool {
    matches!(name, "id" | "tabindex" | "xml:base" | "xml:lang" | "xml:space")
}

fn is_conditional_processing_attr(name: &str) -> bool {
    matches!(name, "requiredExtensions" | "requiredFeatures" | "systemLanguage")
}

fn is_graphical_event_attr(name: &str) -> bool {
    matches!(
        name,
        "onactivate"
            | "onclick"
            | "onfocusin"
            | "onfocusout"
            | "onload"
            | "onmousedown"
            | "onmousemove"
            | "onmouseout"
            | "onmouseover"
            | "onmouseup"
    )
}

fn is_document_event_attr(name: &str) -> bool {
    matches!(
        name,
        "onabort" | "onerror" | "onresize" | "onscroll" | "onunload" | "onzoom"
    )
}

fn is_xlink_attr(name: &str) -> bool {
    matches!(
        name,
        "xlink:actuate"
            | "xlink:arcrole"
            | "xlink:href"
            | "xlink:role"
            | "xlink:show"
            | "xlink:title"
            | "xlink:type"
    )
}

fn is_presentation_default_attr(name: &str) -> bool {
    matches!(
        name,
        "clip"
            | "clip-path"
            | "clip-rule"
            | "mask"
            | "opacity"
            | "stop-color"
            | "stop-opacity"
            | "fill-opacity"
            | "fill-rule"
            | "fill"
            | "stroke"
            | "stroke-width"
            | "stroke-linecap"
            | "stroke-linejoin"
            | "stroke-miterlimit"
            | "stroke-dasharray"
            | "stroke-dashoffset"
            | "stroke-opacity"
            | "paint-order"
            | "vector-effect"
            | "display"
            | "visibility"
            | "marker-start"
            | "marker-mid"
            | "marker-end"
            | "color-interpolation"
            | "color-interpolation-filters"
            | "color-rendering"
            | "shape-rendering"
            | "text-rendering"
            | "image-rendering"
            | "font-style"
            | "font-variant"
            | "font-weight"
            | "font-stretch"
            | "font-size"
            | "font-size-adjust"
            | "letter-spacing"
            | "word-spacing"
            | "text-decoration"
            | "text-anchor"
            | "text-overflow"
            | "writing-mode"
            | "glyph-orientation-vertical"
            | "glyph-orientation-horizontal"
            | "direction"
            | "unicode-bidi"
            | "dominant-baseline"
            | "alignment-baseline"
            | "baseline-shift"
    )
}

#[expect(
    clippy::match_same_arms,
    reason = "Default-value lookup tables are intentionally expressed as direct grouped matches"
)]
fn presentation_default_value(name: &str) -> Option<&'static str> {
    match name {
        "clip" => Some("auto"),
        "clip-path" => Some("none"),
        "clip-rule" => Some("nonzero"),
        "mask" => Some("none"),
        "opacity" => Some("1"),
        "stop-color" => Some("#000"),
        "stop-opacity" => Some("1"),
        "fill-opacity" => Some("1"),
        "fill-rule" => Some("nonzero"),
        "fill" => Some("#000"),
        "stroke" => Some("none"),
        "stroke-width" => Some("1"),
        "stroke-linecap" => Some("butt"),
        "stroke-linejoin" => Some("miter"),
        "stroke-miterlimit" => Some("4"),
        "stroke-dasharray" => Some("none"),
        "stroke-dashoffset" => Some("0"),
        "stroke-opacity" => Some("1"),
        "paint-order" => Some("normal"),
        "vector-effect" => Some("none"),
        "display" => Some("inline"),
        "visibility" => Some("visible"),
        "marker-start" => Some("none"),
        "marker-mid" => Some("none"),
        "marker-end" => Some("none"),
        "color-interpolation" => Some("sRGB"),
        "color-interpolation-filters" => Some("linearRGB"),
        "color-rendering" => Some("auto"),
        "shape-rendering" => Some("auto"),
        "text-rendering" => Some("auto"),
        "image-rendering" => Some("auto"),
        "font-style" => Some("normal"),
        "font-variant" => Some("normal"),
        "font-weight" => Some("normal"),
        "font-stretch" => Some("normal"),
        "font-size" => Some("medium"),
        "font-size-adjust" => Some("none"),
        "letter-spacing" => Some("normal"),
        "word-spacing" => Some("normal"),
        "text-decoration" => Some("none"),
        "text-anchor" => Some("start"),
        "text-overflow" => Some("clip"),
        "writing-mode" => Some("lr-tb"),
        "glyph-orientation-vertical" => Some("auto"),
        "glyph-orientation-horizontal" => Some("0deg"),
        "direction" => Some("ltr"),
        "unicode-bidi" => Some("normal"),
        "dominant-baseline" => Some("auto"),
        "alignment-baseline" => Some("baseline"),
        "baseline-shift" => Some("baseline"),
        _ => None,
    }
}

fn is_non_inheritable_group_presentation_attr(name: &str) -> bool {
    is_presentation_attr(name)
        && !is_inheritable_attr(name)
        && !is_preserved_group_presentation_attr(name)
}

fn is_known_svg_element(name: &str) -> bool {
    matches!(
        name,
        "a"
            | "altGlyph"
            | "altGlyphDef"
            | "altGlyphItem"
            | "animate"
            | "animateColor"
            | "animateMotion"
            | "animateTransform"
            | "circle"
            | "clipPath"
            | "color-profile"
            | "cursor"
            | "defs"
            | "desc"
            | "ellipse"
            | "feBlend"
            | "feColorMatrix"
            | "feComponentTransfer"
            | "feComposite"
            | "feConvolveMatrix"
            | "feDiffuseLighting"
            | "feDisplacementMap"
            | "feDistantLight"
            | "feDropShadow"
            | "feFlood"
            | "feFuncA"
            | "feFuncB"
            | "feFuncG"
            | "feFuncR"
            | "feGaussianBlur"
            | "feImage"
            | "feMerge"
            | "feMergeNode"
            | "feMorphology"
            | "feOffset"
            | "fePointLight"
            | "feSpecularLighting"
            | "feSpotLight"
            | "feTile"
            | "feTurbulence"
            | "filter"
            | "font"
            | "font-face"
            | "font-face-format"
            | "font-face-name"
            | "font-face-src"
            | "font-face-uri"
            | "foreignObject"
            | "g"
            | "glyph"
            | "hkern"
            | "image"
            | "line"
            | "linearGradient"
            | "marker"
            | "mask"
            | "metadata"
            | "missing-glyph"
            | "mpath"
            | "path"
            | "pattern"
            | "polygon"
            | "polyline"
            | "radialGradient"
            | "rect"
            | "script"
            | "set"
            | "solidColor"
            | "stop"
            | "style"
            | "svg"
            | "switch"
            | "symbol"
            | "text"
            | "textPath"
            | "title"
            | "tref"
            | "tspan"
            | "use"
            | "view"
            | "vkern"
    )
}

fn explicit_allowed_children(parent_name: &str) -> Option<&'static [&'static str]> {
    const ANIMATION_DESCRIPTIVE_CHILDREN: &[&str] = &[
        "animate",
        "animateColor",
        "animateMotion",
        "animateTransform",
        "desc",
        "metadata",
        "set",
        "title",
    ];
    const SVG_GROUP_CHILDREN: &[&str] = &[
        "a",
        "altGlyphDef",
        "animate",
        "animateColor",
        "animateMotion",
        "animateTransform",
        "circle",
        "clipPath",
        "color-profile",
        "cursor",
        "defs",
        "desc",
        "ellipse",
        "filter",
        "foreignObject",
        "g",
        "image",
        "line",
        "linearGradient",
        "marker",
        "mask",
        "metadata",
        "path",
        "pattern",
        "polygon",
        "polyline",
        "radialGradient",
        "rect",
        "script",
        "set",
        "solidColor",
        "style",
        "svg",
        "switch",
        "symbol",
        "text",
        "title",
        "use",
        "view",
    ];

    match parent_name {
        "svg" | "g" | "defs" | "symbol" => Some(SVG_GROUP_CHILDREN),
        "circle" | "ellipse" | "line" | "path" | "polygon" | "polyline" | "rect" => {
            Some(ANIMATION_DESCRIPTIVE_CHILDREN)
        }
        _ => None,
    }
}

fn should_remove_unknown_child(doc: &Document, parent_id: usize, child_name: &str) -> bool {
    let Some(parent_name) = node_element_name(doc, parent_id) else {
        return false;
    };
    if parent_name.contains(':') || parent_name == "foreignObject" {
        return false;
    }
    if let Some(allowed) = explicit_allowed_children(parent_name) {
        return !allowed.contains(&child_name);
    }
    !is_known_svg_element(child_name)
}

fn has_attribute_model(name: &str) -> bool {
    matches!(
        name,
        "svg"
            | "g"
            | "rect"
            | "path"
            | "circle"
            | "ellipse"
            | "line"
            | "polygon"
            | "polyline"
            | "style"
            | "defs"
            | "symbol"
            | "use"
            | "text"
            | "tspan"
            | "textPath"
            | "image"
            | "marker"
            | "mask"
            | "pattern"
            | "linearGradient"
            | "radialGradient"
            | "stop"
            | "clipPath"
            | "filter"
    )
}

fn element_allows_presentation(name: &str) -> bool {
    !matches!(name, "style" | "script")
}

#[expect(
    clippy::too_many_lines,
    reason = "The attribute-allowlist table is clearer as one element-to-attrs mapping"
)]
fn attribute_allowed_on_element(element_name: &str, attr_name: &str) -> bool {
    if is_core_attr(attr_name) {
        return true;
    }
    if is_presentation_attr(attr_name) && element_allows_presentation(element_name) {
        return true;
    }
    if is_conditional_processing_attr(attr_name)
        && matches!(
            element_name,
            "svg"
                | "g"
                | "rect"
                | "path"
                | "circle"
                | "ellipse"
                | "line"
                | "polygon"
                | "polyline"
                | "foreignObject"
                | "text"
                | "use"
                | "image"
                | "marker"
                | "mask"
                | "pattern"
                | "linearGradient"
                | "radialGradient"
                | "clipPath"
                | "filter"
        )
    {
        return true;
    }
    if is_graphical_event_attr(attr_name)
        && matches!(
            element_name,
            "svg"
                | "g"
                | "rect"
                | "path"
                | "circle"
                | "ellipse"
                | "line"
                | "polygon"
                | "polyline"
                | "foreignObject"
                | "text"
                | "use"
                | "image"
        )
    {
        return true;
    }
    if element_name == "svg" && is_document_event_attr(attr_name) {
        return true;
    }
    if is_xlink_attr(attr_name)
        && matches!(
            element_name,
            "a" | "image" | "linearGradient" | "radialGradient" | "pattern" | "script" | "use"
        )
    {
        return true;
    }

    match element_name {
        "svg" => matches!(
            attr_name,
            "baseProfile"
                | "class"
                | "contentScriptType"
                | "contentStyleType"
                | "height"
                | "preserveAspectRatio"
                | "style"
                | "version"
                | "viewBox"
                | "width"
                | "x"
                | "y"
                | "zoomAndPan"
        ),
        "g" | "defs" | "symbol" => {
            matches!(attr_name, "class" | "externalResourcesRequired" | "style" | "transform")
        }
        "rect" => matches!(
            attr_name,
            "class"
                | "externalResourcesRequired"
                | "height"
                | "rx"
                | "ry"
                | "style"
                | "transform"
                | "width"
                | "x"
                | "y"
        ),
        "path" => matches!(
            attr_name,
            "class" | "d" | "externalResourcesRequired" | "pathLength" | "style" | "transform"
        ),
        "circle" => matches!(
            attr_name,
            "class" | "cx" | "cy" | "externalResourcesRequired" | "r" | "style" | "transform"
        ),
        "ellipse" => matches!(
            attr_name,
            "class"
                | "cx"
                | "cy"
                | "externalResourcesRequired"
                | "rx"
                | "ry"
                | "style"
                | "transform"
        ),
        "line" => matches!(
            attr_name,
            "class"
                | "externalResourcesRequired"
                | "style"
                | "transform"
                | "x1"
                | "x2"
                | "y1"
                | "y2"
        ),
        "polygon" | "polyline" => matches!(
            attr_name,
            "class" | "externalResourcesRequired" | "points" | "style" | "transform"
        ),
        "style" => matches!(attr_name, "media" | "title" | "type"),
        "use" => matches!(
            attr_name,
            "class"
                | "externalResourcesRequired"
                | "height"
                | "href"
                | "style"
                | "transform"
                | "width"
                | "x"
                | "xlink:href"
                | "y"
        ),
        "text" => matches!(
            attr_name,
            "class"
                | "dx"
                | "dy"
                | "lengthAdjust"
                | "rotate"
                | "style"
                | "textLength"
                | "transform"
                | "x"
                | "y"
        ),
        "tspan" | "textPath" => matches!(
            attr_name,
            "class"
                | "dx"
                | "dy"
                | "href"
                | "lengthAdjust"
                | "rotate"
                | "startOffset"
                | "style"
                | "textLength"
                | "x"
                | "xlink:href"
                | "y"
        ),
        "image" => matches!(
            attr_name,
            "class"
                | "externalResourcesRequired"
                | "height"
                | "href"
                | "preserveAspectRatio"
                | "style"
                | "transform"
                | "width"
                | "x"
                | "xlink:href"
                | "y"
        ),
        "marker" => matches!(
            attr_name,
            "class"
                | "markerHeight"
                | "markerUnits"
                | "markerWidth"
                | "orient"
                | "preserveAspectRatio"
                | "refX"
                | "refY"
                | "style"
                | "viewBox"
        ),
        "mask" => matches!(
            attr_name,
            "class"
                | "height"
                | "maskContentUnits"
                | "maskUnits"
                | "style"
                | "width"
                | "x"
                | "y"
        ),
        "pattern" => matches!(
            attr_name,
            "class"
                | "height"
                | "href"
                | "patternContentUnits"
                | "patternTransform"
                | "patternUnits"
                | "preserveAspectRatio"
                | "style"
                | "viewBox"
                | "width"
                | "x"
                | "xlink:href"
                | "y"
        ),
        "linearGradient" => matches!(
            attr_name,
            "class"
                | "gradientTransform"
                | "gradientUnits"
                | "href"
                | "spreadMethod"
                | "style"
                | "x1"
                | "x2"
                | "xlink:href"
                | "y1"
                | "y2"
        ),
        "radialGradient" => matches!(
            attr_name,
            "class"
                | "cx"
                | "cy"
                | "fr"
                | "fx"
                | "fy"
                | "gradientTransform"
                | "gradientUnits"
                | "href"
                | "r"
                | "spreadMethod"
                | "style"
                | "xlink:href"
        ),
        "stop" => matches!(attr_name, "class" | "offset" | "path" | "style"),
        "clipPath" => matches!(
            attr_name,
            "class" | "clipPathUnits" | "externalResourcesRequired" | "style" | "transform"
        ),
        "filter" => matches!(
            attr_name,
            "class"
                | "externalResourcesRequired"
                | "filterRes"
                | "filterUnits"
                | "height"
                | "primitiveUnits"
                | "style"
                | "width"
                | "x"
                | "y"
        ),
        _ => false,
    }
}

#[expect(
    clippy::match_same_arms,
    reason = "Element default lookup tables are intentionally expressed as direct grouped matches"
)]
fn attribute_default_value(element_name: &str, attr_name: &str) -> Option<&'static str> {
    if attr_name == "xml:space" {
        return Some("default");
    }
    if is_presentation_default_attr(attr_name) && element_allows_presentation(element_name) {
        return presentation_default_value(attr_name);
    }
    match (element_name, attr_name) {
        ("style", "type") => Some("text/css"),
        ("svg", "x") => Some("0"),
        ("svg", "y") => Some("0"),
        ("svg", "width") => Some("100%"),
        ("svg", "height") => Some("100%"),
        ("svg", "preserveAspectRatio") => Some("xMidYMid meet"),
        ("svg", "zoomAndPan") => Some("magnify"),
        ("svg", "version") => Some("1.1"),
        ("svg", "baseProfile") => Some("none"),
        ("svg", "contentScriptType") => Some("application/ecmascript"),
        ("svg", "contentStyleType") => Some("text/css"),
        ("rect", "x") => Some("0"),
        ("rect", "y") => Some("0"),
        _ => None,
    }
}

fn is_in_foreign_object_subtree(doc: &Document, node_id: usize) -> bool {
    has_ancestor_element(doc, node_id, "foreignObject")
}

fn has_ancestor_element(doc: &Document, node_id: usize, name: &str) -> bool {
    let mut current = Some(node_id);
    while let Some(id) = current {
        let Some(element) = node_element(doc, id) else {
            current = doc.node(id).parent;
            continue;
        };
        if element.name == name {
            return true;
        }
        current = doc.node(id).parent;
    }
    false
}

fn collect_semantic_stylesheet(doc: &Document) -> Vec<StylesheetRule> {
    let mut rules = Vec::new();
    for node_id in 1..doc.nodes.len() {
        let Some(element) = node_element(doc, node_id) else {
            continue;
        };
        if element.name != "style" || is_in_foreign_object_subtree(doc, node_id) {
            continue;
        }
        for child_id in doc.children(node_id) {
            let NodeKind::Text(text) = &doc.node(child_id).kind else {
                continue;
            };
            rules.extend(parse_stylesheet_rules(text));
        }
    }
    rules
}

fn collect_style_css(doc: &Document, node_id: usize) -> (String, bool) {
    let mut css = String::new();
    let mut saw_cdata = false;
    for child_id in doc.children(node_id) {
        match &doc.node(child_id).kind {
            NodeKind::Text(text) => css.push_str(text),
            NodeKind::Cdata(text) => {
                saw_cdata = true;
                css.push_str(text);
            }
            _ => {}
        }
    }
    (css, saw_cdata)
}

fn replace_children_with_style_content(
    doc: &mut Document,
    node_id: usize,
    css: &str,
    content_kind: StyleContentKind,
) {
    let child_ids: Vec<_> = doc.children(node_id).collect();
    for child_id in child_ids {
        doc.node_mut(child_id).parent = None;
        doc.node_mut(child_id).next_sibling = None;
    }
    doc.node_mut(node_id).first_child = None;
    doc.node_mut(node_id).last_child = None;
    match content_kind {
        StyleContentKind::Text => {
            doc.append_child(node_id, NodeKind::Text(css.to_string()));
        }
        StyleContentKind::Cdata => {
            doc.append_child(node_id, NodeKind::Cdata(css.to_string()));
        }
    }
}

fn stylesheet_mentions_attr_selector(stylesheet: &[StylesheetRule], name: &str) -> bool {
    let pattern = format!("[{name}");
    stylesheet
        .iter()
        .any(|rule| rule.selector.contains(pattern.as_str()))
}

fn compute_static_style(
    doc: &Document,
    node_id: usize,
    stylesheet: &[StylesheetRule],
    cache: &mut HashMap<usize, HashMap<String, String>>,
) -> HashMap<String, String> {
    if let Some(cached) = cache.get(&node_id) {
        return cached.clone();
    }

    let mut style = HashMap::new();
    let Some(element) = node_element(doc, node_id) else {
        return style;
    };

    if let Some(parent_id) = doc.node(node_id).parent
        && matches!(doc.node(parent_id).kind, NodeKind::Element(_))
    {
        for (name, value) in compute_static_style(doc, parent_id, stylesheet, cache) {
            if is_inheritable_attr(name.as_str()) {
                style.insert(name, value);
            }
        }
    }

    for attribute in &element.attributes {
        if is_presentation_attr(attribute.name.as_str()) {
            style.insert(attribute.name.clone(), attribute.value.clone());
        }
    }

    let mut matched_rules = stylesheet
        .iter()
        .enumerate()
        .filter(|(_, rule)| selector_matches(doc, node_id, rule.selector.as_str()))
        .collect::<Vec<_>>();
    matched_rules.sort_by(|(left_index, left), (right_index, right)| {
        left.specificity
            .cmp(&right.specificity)
            .then(left_index.cmp(right_index))
    });
    for (_, rule) in matched_rules {
        apply_style_declarations(&mut style, &rule.declarations);
    }

    if let Some(style_attr) = attribute_value(element.attributes.as_slice(), "style") {
        apply_style_declarations(&mut style, &parse_style_declarations(style_attr));
    }

    cache.insert(node_id, style.clone());
    style
}

fn apply_style_declarations(
    target: &mut HashMap<String, String>,
    declarations: &[StyleDeclaration],
) {
    for declaration in declarations {
        target.insert(declaration.name.clone(), declaration.value.clone());
    }
}

fn collect_reference_ids(doc: &Document) -> HashSet<String> {
    let mut references = HashSet::new();
    for node in &doc.nodes {
        let NodeKind::Element(element) = &node.kind else {
            continue;
        };
        for attribute in &element.attributes {
            collect_references_from_value(attribute.value.as_str(), &mut references);
        }
    }
    references
}

#[derive(Debug, Clone)]
struct CleanupIdsParams {
    remove: bool,
    minify: bool,
    preserve: HashSet<String>,
    preserve_prefixes: Vec<String>,
    force: bool,
}

const GENERATED_ID_CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

fn cleanup_ids(doc: &mut Document, params: Option<&Value>) {
    let params = cleanup_ids_params(params);
    if !params.force {
        if document_has_style_or_scripts(doc) {
            return;
        }
        let root_children: Vec<_> = doc.children(doc.root_id()).collect();
        if !root_children.is_empty()
            && root_children
                .iter()
                .all(|child_id| node_element_name(doc, *child_id) == Some("defs"))
        {
            return;
        }
    }

    let mut node_by_id = HashMap::<String, usize>::new();
    let mut duplicate_nodes = Vec::<usize>::new();
    let mut references_by_id = HashMap::<String, Vec<(usize, String)>>::new();

    for node_id in 1..doc.nodes.len() {
        let Some(element) = node_element(doc, node_id) else {
            continue;
        };
        for attribute in &element.attributes {
            if attribute.name == "id" {
                if node_by_id.contains_key(attribute.value.as_str()) {
                    duplicate_nodes.push(node_id);
                } else {
                    node_by_id.insert(attribute.value.clone(), node_id);
                }
                continue;
            }
            collect_reference_pairs(
                node_id,
                attribute.name.as_str(),
                attribute.value.as_str(),
                &mut references_by_id,
            );
        }
    }

    for node_id in duplicate_nodes {
        let NodeKind::Element(element) = &mut doc.node_mut(node_id).kind else {
            continue;
        };
        element.attributes.retain(|attribute| attribute.name != "id");
    }

    let mut current_id = None;
    let referenced_ids = references_by_id.keys().cloned().collect::<Vec<_>>();
    for id in referenced_ids {
        let Some(node_id) = node_by_id.remove(id.as_str()) else {
            continue;
        };
        if params.minify && !cleanup_ids_preserved(&params, id.as_str()) {
            let mut next_id;
            loop {
                current_id = Some(generate_next_id(current_id.as_deref()));
                next_id = current_id.clone().unwrap_or_default();
                if cleanup_ids_preserved(&params, next_id.as_str()) {
                    continue;
                }
                if references_by_id.contains_key(next_id.as_str())
                    && !node_by_id.contains_key(next_id.as_str())
                {
                    continue;
                }
                break;
            }

            let NodeKind::Element(element) = &mut doc.node_mut(node_id).kind else {
                continue;
            };
            set_or_push_attribute(
                &mut element.attributes,
                "id",
                next_id.as_str(),
                QuoteStyle::Double,
            );
            if let Some(references) = references_by_id.get(id.as_str()) {
                for (reference_node_id, attribute_name) in references {
                    rewrite_reference_attribute(
                        doc,
                        *reference_node_id,
                        attribute_name.as_str(),
                        id.as_str(),
                        next_id.as_str(),
                    );
                }
            }
        }
    }

    if params.remove {
        for (id, node_id) in node_by_id {
            if cleanup_ids_preserved(&params, id.as_str()) {
                continue;
            }
            let NodeKind::Element(element) = &mut doc.node_mut(node_id).kind else {
                continue;
            };
            element.attributes.retain(|attribute| attribute.name != "id");
        }
    }
}

fn cleanup_ids_params(params: Option<&Value>) -> CleanupIdsParams {
    CleanupIdsParams {
        remove: json_bool(params, "remove", true),
        minify: json_bool(params, "minify", true),
        preserve: params
            .and_then(|value| value.get("preserve"))
            .map(json_string_or_array)
            .unwrap_or_default()
            .into_iter()
            .collect(),
        preserve_prefixes: params
            .and_then(|value| value.get("preservePrefixes"))
            .map(json_string_or_array)
            .unwrap_or_default(),
        force: json_bool(params, "force", false),
    }
}

fn json_string_or_array(value: &Value) -> Vec<String> {
    if let Some(items) = value.as_array() {
        return items
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect();
    }
    value.as_str().map(|item| vec![item.to_string()]).unwrap_or_default()
}

fn cleanup_ids_preserved(params: &CleanupIdsParams, id: &str) -> bool {
    params.preserve.contains(id)
        || params
            .preserve_prefixes
            .iter()
            .any(|prefix| id.starts_with(prefix))
}

fn generate_next_id(current: Option<&str>) -> String {
    let mut digits = current
        .map(|value| {
            value
                .bytes()
                .filter_map(|byte| GENERATED_ID_CHARS.iter().position(|ch| *ch == byte))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if digits.is_empty() {
        digits.push(0);
    } else {
        let mut index = digits.len() - 1;
        loop {
            digits[index] += 1;
            if digits[index] < GENERATED_ID_CHARS.len() {
                break;
            }
            digits[index] = 0;
            if index == 0 {
                digits.insert(0, 0);
                break;
            }
            index -= 1;
        }
    }

    digits
        .into_iter()
        .map(|index| char::from(GENERATED_ID_CHARS[index]))
        .collect()
}

fn collect_reference_pairs(
    node_id: usize,
    attribute_name: &str,
    value: &str,
    references_by_id: &mut HashMap<String, Vec<(usize, String)>>,
) {
    let mut references = HashSet::new();
    collect_references_from_value(value, &mut references);
    if attribute_name == "begin" {
        collect_begin_references(value, &mut references);
    }
    for id in references {
        references_by_id
            .entry(id)
            .or_default()
            .push((node_id, attribute_name.to_string()));
    }
}

fn collect_begin_references(value: &str, references: &mut HashSet<String>) {
    for item in value.split(';').map(str::trim) {
        let Some((candidate, _)) = item.split_once('.') else {
            continue;
        };
        if !candidate.is_empty()
            && candidate
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':'))
        {
            references.insert(candidate.to_string());
        }
    }
}

fn rewrite_reference_attribute(
    doc: &mut Document,
    node_id: usize,
    attribute_name: &str,
    old_id: &str,
    new_id: &str,
) {
    let NodeKind::Element(element) = &mut doc.node_mut(node_id).kind else {
        return;
    };
    let Some(attribute) = attribute_named_mut(element.attributes.as_mut_slice(), attribute_name) else {
        return;
    };
    attribute.value = rewrite_reference_value(attribute.value.as_str(), attribute_name, old_id, new_id);
}

fn rewrite_reference_value(value: &str, attribute_name: &str, old_id: &str, new_id: &str) -> String {
    let mut rewritten = value.replace(format!("#{old_id}").as_str(), format!("#{new_id}").as_str());
    if attribute_name == "begin" && !rewritten.contains('#') {
        rewritten = rewritten
            .split(';')
            .map(str::trim)
            .map(|item| {
                item.strip_prefix(format!("{old_id}.").as_str()).map_or_else(
                    || item.to_string(),
                    |tail| format!("{new_id}.{tail}"),
                )
            })
            .collect::<Vec<_>>()
            .join(";");
    }
    rewritten
}

fn collect_use_references(doc: &Document) -> HashMap<String, Vec<usize>> {
    let mut references = HashMap::<String, Vec<usize>>::new();
    for (node_id, node) in doc.nodes.iter().enumerate().skip(1) {
        let NodeKind::Element(element) = &node.kind else {
            continue;
        };
        if element.name != "use" {
            continue;
        }
        for attribute in &element.attributes {
            if attribute.name != "href" && !attribute.name.ends_with(":href") {
                continue;
            }
            if let Some(id) = attribute.value.strip_prefix('#') {
                references.entry(id.to_string()).or_default().push(node_id);
            }
        }
    }
    references
}

fn collect_references_from_value(value: &str, references: &mut HashSet<String>) {
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'#' {
            index += 1;
            continue;
        }
        index += 1;
        let start = index;
        while index < bytes.len() {
            let char = char::from(bytes[index]);
            if char.is_ascii_alphanumeric() || matches!(char, '_' | '-' | '.' | ':') {
                index += 1;
            } else {
                break;
            }
        }
        if start < index {
            references.insert(value[start..index].to_string());
        }
    }
}

fn can_remove_non_rendering_node(
    doc: &Document,
    node_id: usize,
    references: &HashSet<String>,
) -> bool {
    !node_has_referenced_id(doc, node_id, references)
}

fn record_removed_def_id(doc: &Document, node_id: usize, removed_def_ids: &mut HashSet<String>) {
    let Some(parent_id) = doc.node(node_id).parent else {
        return;
    };
    if node_element_name(doc, parent_id) != Some("defs") {
        return;
    }
    if let Some(id) = node_element(doc, node_id)
        .and_then(|element| attribute_value(element.attributes.as_slice(), "id"))
    {
        removed_def_ids.insert(id.to_string());
    }
}

fn node_has_referenced_id(doc: &Document, node_id: usize, references: &HashSet<String>) -> bool {
    if node_element(doc, node_id)
        .and_then(|element| attribute_value(element.attributes.as_slice(), "id"))
        .is_some_and(|id| references.contains(id))
    {
        return true;
    }
    doc.children(node_id)
        .any(|child_id| node_has_referenced_id(doc, child_id, references))
}

fn has_visible_descendant_attr(doc: &Document, node_id: usize) -> bool {
    doc.children(node_id).any(|child_id| {
        if node_element(doc, child_id).is_some_and(|element| {
            attribute_value(element.attributes.as_slice(), "visibility") == Some("visible")
        }) {
            return true;
        }
        has_visible_descendant_attr(doc, child_id)
    })
}

fn should_remove_path(attributes: &[Attribute], computed_style: &HashMap<String, String>) -> bool {
    let Some(d) = attribute_value(attributes, "d") else {
        return true;
    };
    if d.trim().is_empty() {
        return true;
    }
    let Ok(path_data) = parse_path_commands(d) else {
        return false;
    };
    if path_data.is_empty() {
        return true;
    }
    path_data.len() == 1
        && matches!(path_data[0], PathCommand::MoveTo { .. })
        && !computed_style.contains_key("marker-start")
        && !computed_style.contains_key("marker-end")
}

fn hidden_elems_deoptimized(doc: &Document) -> bool {
    doc.nodes.iter().enumerate().skip(1).any(|(node_id, node)| {
        let NodeKind::Element(element) = &node.kind else {
            return false;
        };
        (element.name == "style" && doc.children(node_id).next().is_some())
            || element_has_scripts(doc, node_id)
    })
}

fn is_preserved_group_presentation_attr(name: &str) -> bool {
    matches!(
        name,
        "clip-path"
            | "display"
            | "filter"
            | "mask"
            | "opacity"
            | "text-decoration"
            | "transform"
            | "unicode-bidi"
    )
}

fn is_reference_property(name: &str) -> bool {
    matches!(
        name,
        "clip-path"
            | "color-profile"
            | "fill"
            | "filter"
            | "marker-end"
            | "marker-mid"
            | "marker-start"
            | "mask"
            | "stroke"
            | "style"
    )
}

fn has_animated_attr(doc: &Document, node_id: usize, name: &str) -> bool {
    let NodeKind::Element(element) = &doc.node(node_id).kind else {
        return false;
    };
    if is_animation_element(element.name.as_str())
        && attribute_value(element.attributes.as_slice(), "attributeName") == Some(name)
    {
        return true;
    }
    for child_id in doc.children(node_id) {
        if has_animated_attr(doc, child_id, name) {
            return true;
        }
    }
    false
}

fn is_animation_element(name: &str) -> bool {
    matches!(
        name,
        "animate" | "animateColor" | "animateMotion" | "animateTransform" | "set"
    )
}

fn group_has_animation_children(doc: &Document, group_id: usize) -> bool {
    doc.children(group_id)
        .any(|child_id| match &doc.node(child_id).kind {
            NodeKind::Element(child) => is_animation_element(child.name.as_str()),
            _ => false,
        })
}

fn replace_node_with_children(doc: &mut Document, node_id: usize) {
    let Some(parent_id) = doc.node(node_id).parent else {
        return;
    };
    let parent_children: Vec<_> = doc.children(parent_id).collect();
    let replacement_children: Vec<_> = doc.children(node_id).collect();
    let mut reordered = Vec::with_capacity(parent_children.len() + replacement_children.len());
    for child_id in parent_children {
        if child_id == node_id {
            reordered.extend(replacement_children.iter().copied());
        } else {
            reordered.push(child_id);
        }
    }
    doc.reorder_children(parent_id, &reordered);
    doc.node_mut(node_id).parent = None;
    doc.node_mut(node_id).first_child = None;
    doc.node_mut(node_id).last_child = None;
    doc.node_mut(node_id).next_sibling = None;
}

fn collapse_attribute_newlines(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = String::with_capacity(value.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\r' || bytes[index] == b'\n' {
            let prev = out.chars().next_back();
            if bytes[index] == b'\r' && bytes.get(index + 1) == Some(&b'\n') {
                index += 1;
            }
            let mut next_index = index + 1;
            while matches!(bytes.get(next_index), Some(b'\r' | b'\n')) {
                next_index += 1;
            }
            let next = bytes.get(next_index).copied();
            if prev.is_some_and(|char| !char.is_whitespace())
                && next.is_some_and(|byte| !char::from(byte).is_whitespace())
            {
                out.push(' ');
            }
            index = next_index;
            continue;
        }
        out.push(char::from(bytes[index]));
        index += 1;
    }
    out
}

fn collect_useful_children(doc: &Document, node_id: usize, useful_children: &mut Vec<usize>) {
    for child_id in doc.children(node_id) {
        let NodeKind::Element(element) = &doc.node(child_id).kind else {
            continue;
        };
        if element
            .attributes
            .iter()
            .any(|attribute| attribute.name == "id")
            || element.name == "style"
        {
            useful_children.push(child_id);
        } else {
            collect_useful_children(doc, child_id, useful_children);
        }
    }
}

fn is_non_rendering(name: &str) -> bool {
    matches!(
        name,
        "clipPath"
            | "filter"
            | "linearGradient"
            | "marker"
            | "mask"
            | "pattern"
            | "radialGradient"
            | "solidColor"
            | "symbol"
    )
}

fn is_container_element(name: &str) -> bool {
    matches!(
        name,
        "a" | "defs"
            | "foreignObject"
            | "g"
            | "marker"
            | "mask"
            | "missing-glyph"
            | "pattern"
            | "svg"
            | "switch"
            | "symbol"
    )
}

fn collapse_repeating_spaces(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut previous_space = false;
    for char in value.chars() {
        if char == ' ' {
            if !previous_space {
                out.push(char);
            }
            previous_space = true;
        } else {
            out.push(char);
            previous_space = false;
        }
    }
    out
}

fn default_sort_attr_order() -> Vec<String> {
    [
        "id", "width", "height", "x", "x1", "x2", "y", "y1", "y2", "cx", "cy", "r", "fill",
        "stroke", "marker", "d", "points",
    ]
    .iter()
    .map(|value| (*value).to_string())
    .collect()
}

fn compare_attrs(
    left: &str,
    right: &str,
    order: &[String],
    xmlns_order: &str,
) -> std::cmp::Ordering {
    let left_priority = namespace_priority(left, xmlns_order);
    let right_priority = namespace_priority(right, xmlns_order);
    match right_priority.cmp(&left_priority) {
        std::cmp::Ordering::Equal => {}
        ordering => return ordering,
    }

    let left_part = left.split('-').next().unwrap_or(left);
    let right_part = right.split('-').next().unwrap_or(right);
    if left_part != right_part {
        let left_index = order.iter().position(|item| item == left_part);
        let right_index = order.iter().position(|item| item == right_part);
        match (left_index, right_index) {
            (Some(left_index), Some(right_index)) => match left_index.cmp(&right_index) {
                std::cmp::Ordering::Equal => {}
                ordering => return ordering,
            },
            (Some(_), None) => return std::cmp::Ordering::Less,
            (None, Some(_)) => return std::cmp::Ordering::Greater,
            (None, None) => {}
        }
    }

    left.cmp(right)
}

fn namespace_priority(name: &str, xmlns_order: &str) -> usize {
    if xmlns_order == "front" {
        if name == "xmlns" {
            return 3;
        }
        if name.starts_with("xmlns:") {
            return 2;
        }
    }
    if name.contains(':') {
        return 1;
    }
    0
}

fn editor_namespaces() -> Vec<String> {
    vec![
        "http://creativecommons.org/ns#".to_string(),
        "http://inkscape.sourceforge.net/DTD/sodipodi-0.dtd".to_string(),
        "http://krita.org/namespaces/svg/krita".to_string(),
        "http://ns.adobe.com/AdobeIllustrator/10.0/".to_string(),
        "http://ns.adobe.com/AdobeSVGViewerExtensions/3.0/".to_string(),
        "http://ns.adobe.com/Extensibility/1.0/".to_string(),
        "http://ns.adobe.com/Flows/1.0/".to_string(),
        "http://ns.adobe.com/GenericCustomNamespace/1.0/".to_string(),
        "http://ns.adobe.com/Graphs/1.0/".to_string(),
        "http://ns.adobe.com/ImageReplacement/1.0/".to_string(),
        "http://ns.adobe.com/SaveForWeb/1.0/".to_string(),
        "http://ns.adobe.com/Variables/1.0/".to_string(),
        "http://ns.adobe.com/XPath/1.0/".to_string(),
        "http://purl.org/dc/elements/1.1/".to_string(),
        "http://schemas.microsoft.com/visio/2003/SVGExtensions/".to_string(),
        "http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd".to_string(),
        "http://taptrix.com/vectorillustrator/svg_extensions".to_string(),
        "http://www.bohemiancoding.com/sketch/ns".to_string(),
        "http://www.figma.com/figma/ns".to_string(),
        "http://www.inkscape.org/namespaces/inkscape".to_string(),
        "http://www.serif.com/".to_string(),
        "http://www.vector.evaxdesign.sk".to_string(),
        "http://www.w3.org/1999/02/22-rdf-syntax-ns#".to_string(),
        "https://boxy-svg.com".to_string(),
    ]
}

fn find_root_svg(doc: &Document) -> Option<usize> {
    doc.children(doc.root_id()).find(|id| {
        matches!(
            &doc.node(*id).kind,
            NodeKind::Element(element) if element.name == "svg"
        )
    })
}

const fn matches_doctype(kind: &NodeKind) -> bool {
    matches!(kind, NodeKind::Doctype(_))
}

const fn matches_xml_decl(kind: &NodeKind) -> bool {
    matches!(kind, NodeKind::XmlDecl(_))
}

fn remove_by(doc: &mut Document, predicate: impl Fn(&NodeKind) -> bool) {
    let mut ids = Vec::new();
    for (id, node) in doc.nodes.iter().enumerate().skip(1) {
        if predicate(&node.kind) {
            ids.push(id);
        }
    }
    for id in ids {
        detach_node(doc, id);
    }
}

fn detach_node(doc: &mut Document, id: usize) {
    let Some(parent) = doc.nodes[id].parent else {
        return;
    };
    let mut previous: Option<usize> = None;
    let mut cursor = doc.nodes[parent].first_child;
    while let Some(current) = cursor {
        if current == id {
            let next = doc.nodes[current].next_sibling;
            if let Some(previous) = previous {
                doc.nodes[previous].next_sibling = next;
            } else {
                doc.nodes[parent].first_child = next;
            }
            if doc.nodes[parent].last_child == Some(id) {
                doc.nodes[parent].last_child = previous;
            }
            doc.nodes[id].parent = None;
            doc.nodes[id].next_sibling = None;
            break;
        }
        previous = Some(current);
        cursor = doc.nodes[current].next_sibling;
    }
}
