use std::collections::HashSet;

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
    "removeUselessDefs",
    "removeEmptyText",
    "moveElemsAttrsToGroup",
    "moveGroupAttrsToElems",
    "removeEmptyAttrs",
    "removeEmptyContainers",
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
            "removeUselessDefs" => remove_useless_defs(doc),
            "removeEmptyText" => remove_empty_text(doc, params.as_ref()),
            "moveElemsAttrsToGroup" => move_elems_attrs_to_group(doc),
            "moveGroupAttrsToElems" => move_group_attrs_to_elems(doc),
            "removeEmptyAttrs" => remove_empty_attrs(doc),
            "removeEmptyContainers" => remove_empty_containers(doc),
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

    if name == "g" && group_may_render(element.attributes.as_slice(), stylesheet) {
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

fn group_may_render(attributes: &[Attribute], stylesheet: &FilterStylesheet) -> bool {
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
