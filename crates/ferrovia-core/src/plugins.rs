use serde_json::Value;

use crate::ast::{Attribute, Document, NodeKind, QuoteStyle};
use crate::config::{Config, PluginSpec};
use crate::error::{FerroviaError, Result};

const PRESET_DEFAULT: &[&str] = &[
    "removeDoctype",
    "removeXMLProcInst",
    "removeComments",
    "removeDeprecatedAttrs",
    "removeMetadata",
    "removeEditorsNSData",
    "cleanupAttrs",
    "removeEmptyText",
    "removeEmptyAttrs",
    "removeUnusedNS",
    "sortAttrs",
    "sortDefsChildren",
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
            "removeEmptyText" => remove_empty_text(doc, params.as_ref()),
            "removeEmptyAttrs" => remove_empty_attrs(doc),
            "removeUnusedNS" => remove_unused_ns(doc),
            "sortAttrs" => sort_attrs(doc, params.as_ref()),
            "sortDefsChildren" => sort_defs_children(doc),
            "removeTitle" => remove_elements(doc, "title"),
            "removeDesc" => remove_desc(doc, params.as_ref()),
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

fn remove_elements(doc: &mut Document, name: &str) {
    remove_by(
        doc,
        |kind| matches!(kind, NodeKind::Element(element) if element.name == name),
    );
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
