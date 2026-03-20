use serde_json::Value;

use crate::ast::{Attribute, Document, NodeKind, QuoteStyle};
use crate::config::{Config, PluginSpec};
use crate::error::{FerroviaError, Result};

const PRESET_DEFAULT: &[&str] = &[
    "removeDoctype",
    "removeXMLProcInst",
    "removeComments",
    "removeMetadata",
];

pub fn apply_plugins(doc: &mut Document, config: &Config) -> Result<()> {
    for plugin in expand_plugins(config)? {
        let name = plugin.name().to_string();
        let params = plugin.params().cloned();
        match name.as_str() {
            "removeDoctype" => remove_by(doc, matches_doctype),
            "removeXMLProcInst" => remove_by(doc, matches_xml_decl),
            "removeComments" => remove_comments(doc, params.as_ref()),
            "removeMetadata" => remove_elements(doc, "metadata"),
            "removeTitle" => remove_elements(doc, "title"),
            "removeDesc" => remove_desc(doc, params.as_ref()),
            "removeDimensions" => remove_dimensions(doc),
            "removeXMLNS" => remove_xmlns(doc),
            other => return Err(FerroviaError::UnsupportedPlugin(other.to_string())),
        }
    }
    Ok(())
}

fn expand_plugins(config: &Config) -> Result<Vec<PluginSpec>> {
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
                            }))
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
    Ok(expanded)
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
    let normalized = text.trim();
    normalized.is_empty()
        || normalized.contains("Created with")
        || normalized.contains("Created using")
        || normalized.contains("Generator:")
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

fn find_root_svg(doc: &Document) -> Option<usize> {
    doc.children(doc.root_id()).find(|id| {
        matches!(
            &doc.node(*id).kind,
            NodeKind::Element(element) if element.name == "svg"
        )
    })
}

fn matches_doctype(kind: &NodeKind) -> bool {
    matches!(kind, NodeKind::Doctype(_))
}

fn matches_xml_decl(kind: &NodeKind) -> bool {
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
