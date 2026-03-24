use std::collections::{HashMap, HashSet};

use serde_json::Value;

use crate::ast::{Attribute, Document, NodeId, NodeKind, QuoteStyle};
use crate::config::Config;
use crate::error::Result;
use crate::optimize::OptimizeResult;
use crate::parser::parse;
use crate::plugins::{apply_named_plugin, expand_plugins};
use crate::serializer::serialize;
use crate::style::{
    CssRule, StyleDeclaration, parse_css_rules, parse_style_declarations, selector_matches,
    selector_specificity, serialize_css_rules, update_style_attribute,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpikePluginFamily {
    CleanupIds,
    ConvertPathData,
    InlineStyles,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectPortSpikeConfig {
    pub families: Vec<SpikePluginFamily>,
}

impl Default for DirectPortSpikeConfig {
    fn default() -> Self {
        Self {
            families: vec![
                SpikePluginFamily::CleanupIds,
                SpikePluginFamily::ConvertPathData,
                SpikePluginFamily::InlineStyles,
            ],
        }
    }
}

impl DirectPortSpikeConfig {
    #[must_use]
    pub fn includes(&self, family: SpikePluginFamily) -> bool {
        self.families.contains(&family)
    }
}

/// Optimize an SVG string with the semantic direct-port spike enabled for the
/// selected plugin families.
///
/// # Errors
///
/// Returns an error when parsing fails or when the supplied config references a
/// plugin not implemented by the current ferrovia build.
pub fn optimize_with_direct_port_spike(
    svg: &str,
    config: &Config,
    spike: &DirectPortSpikeConfig,
) -> Result<OptimizeResult> {
    let mut current = svg.to_string();
    let passes = if config.multipass { 10 } else { 1 };

    for _ in 0..passes {
        let mut doc = parse(&current)?;
        apply_spike_plugins(&mut doc, config, spike)?;
        let next = serialize(&doc, &config.js2svg);
        if next == current {
            current = next;
            break;
        }
        current = next;
    }

    Ok(OptimizeResult { data: current })
}

fn apply_spike_plugins(
    doc: &mut Document,
    config: &Config,
    spike: &DirectPortSpikeConfig,
) -> Result<()> {
    for plugin in expand_plugins(config) {
        let name = plugin.name().to_string();
        let params = plugin.params().cloned();
        match name.as_str() {
            "cleanupIds" if spike.includes(SpikePluginFamily::CleanupIds) => {
                cleanup_ids_direct_port(doc, params.as_ref());
            }
            "inlineStyles" if spike.includes(SpikePluginFamily::InlineStyles) => {
                inline_styles_direct_port(doc, params.as_ref());
            }
            "convertPathData" if spike.includes(SpikePluginFamily::ConvertPathData) => {
                convert_path_data_direct_port(doc, params.as_ref())?;
            }
            _ => apply_named_plugin(doc, name.as_str(), params.as_ref())?,
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct CleanupIdsParams {
    remove: bool,
    minify: bool,
    preserve: HashSet<String>,
    preserve_prefixes: Vec<String>,
    force: bool,
}

#[derive(Debug, Clone)]
struct CleanupReference {
    node_id: NodeId,
    attribute_name: String,
}

const GENERATED_ID_CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

fn cleanup_ids_direct_port(doc: &mut Document, params: Option<&Value>) {
    let params = cleanup_ids_params(params);

    if !params.force {
        if document_has_style_or_scripts(doc) {
            return;
        }
        if svg_has_defs_only(doc) {
            return;
        }
    }

    let mut node_by_id = HashMap::<String, NodeId>::new();
    let mut references_by_id = HashMap::<String, Vec<CleanupReference>>::new();
    let mut reference_order = Vec::<String>::new();
    let mut duplicate_nodes = Vec::<NodeId>::new();

    for node_id in walk_element_ids(doc) {
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
            for reference in compat_find_references(attribute.name.as_str(), attribute.value.as_str()) {
                references_by_id
                    .entry(reference.clone())
                    .or_insert_with(|| {
                        reference_order.push(reference.clone());
                        Vec::new()
                    })
                    .push(CleanupReference {
                        node_id,
                        attribute_name: attribute.name.clone(),
                    });
            }
        }
    }

    for node_id in duplicate_nodes {
        let NodeKind::Element(element) = &mut doc.node_mut(node_id).kind else {
            continue;
        };
        element.attributes.retain(|attribute| attribute.name != "id");
    }

    let mut current_id = None;
    for id in reference_order {
        let Some(node_id) = node_by_id.get(id.as_str()).copied() else {
            continue;
        };
        if params.minify && !cleanup_ids_preserved(&params, id.as_str()) {
            let next_id = loop {
                let generated = generate_next_id(current_id.as_deref());
                current_id = Some(generated.clone());
                if cleanup_ids_preserved(&params, generated.as_str()) {
                    continue;
                }
                if references_by_id.contains_key(generated.as_str())
                    && !node_by_id.contains_key(generated.as_str())
                {
                    continue;
                }
                break generated;
            };

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
                for reference in references {
                    rewrite_reference_attribute(
                        doc,
                        reference.node_id,
                        reference.attribute_name.as_str(),
                        id.as_str(),
                        next_id.as_str(),
                    );
                }
            }
        }
        node_by_id.remove(id.as_str());
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

fn convert_path_data_direct_port(doc: &mut Document, params: Option<&Value>) -> Result<()> {
    // The spike keeps the current path backend for the first decision pass while
    // moving orchestration and helper boundaries into `svgo_spike`.
    apply_named_plugin(doc, "convertPathData", params)
}

#[derive(Debug, Clone)]
struct InlineStylesParams {
    only_matched_once: bool,
    remove_matched_selectors: bool,
    use_mqs: Vec<String>,
}

#[derive(Debug, Clone)]
struct InlineSelectorEntry {
    style_node_id: NodeId,
    rule_index: usize,
    selector: String,
    specificity: [u8; 3],
    order: usize,
    matched_elements: Vec<NodeId>,
}

#[expect(
    clippy::too_many_lines,
    reason = "The direct-port spike intentionally keeps inlineStyles orchestration in one SVGO-shaped pass"
)]
fn inline_styles_direct_port(doc: &mut Document, params: Option<&Value>) {
    let params = inline_styles_params(params);
    let mut stylesheets = HashMap::<NodeId, Vec<CssRule>>::new();
    let mut selectors = Vec::<InlineSelectorEntry>::new();
    let mut order = 0usize;

    for node_id in walk_element_ids(doc) {
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

        let css = collect_style_css(doc, node_id);
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

        for (rule_index, rule) in rules.iter().enumerate() {
            if rule
                .media_query
                .as_deref()
                .is_some_and(|query| !params.use_mqs.iter().any(|allowed| allowed == query))
            {
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
    }

    selectors.sort_by(|left, right| {
        left.specificity
            .cmp(&right.specificity)
            .then(left.order.cmp(&right.order))
            .reverse()
    });

    for selector in &mut selectors {
        let matched_elements = query_selector_all(doc, selector.selector.as_str());
        if matched_elements.is_empty() {
            continue;
        }
        selector.matched_elements.clone_from(&matched_elements);
        if params.only_matched_once && matched_elements.len() > 1 {
            continue;
        }
        let declarations = stylesheets
            .get(&selector.style_node_id)
            .and_then(|rules| rules.get(selector.rule_index))
            .map(|rule| rule.declarations.clone())
            .unwrap_or_default();
        let remaining_selectors = collect_remaining_selectors(&stylesheets);
        for node_id in matched_elements {
            inline_rule_declarations(doc, node_id, &declarations, &remaining_selectors);
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
            let selector_classes = extract_selector_tokens(selector.selector.as_str(), '.');
            let selector_ids = extract_selector_tokens(selector.selector.as_str(), '#');
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
        replace_children_with_text(doc, style_node_id, compact_css.as_str());
    }
}

fn inline_rule_declarations(
    doc: &mut Document,
    node_id: NodeId,
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
            && !remaining_selectors
                .iter()
                .any(|selector| selector.contains(format!("[{}", declaration.name).as_str()))
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

fn cleanup_inlined_selector_attrs(
    doc: &mut Document,
    node_id: NodeId,
    selector_classes: &[String],
    selector_ids: &[String],
    remaining_selectors: &[String],
) {
    let NodeKind::Element(element) = &mut doc.node_mut(node_id).kind else {
        return;
    };
    if !selector_ids.is_empty()
        && let Some(id_attr) = attribute_named(element.attributes.as_slice(), "id")
        && selector_ids.iter().any(|selector_id| selector_id == &id_attr.value)
        && !remaining_selectors.iter().any(|selector| selector.contains('#'))
    {
        element.attributes.retain(|attribute| attribute.name != "id");
    }

    if !selector_classes.is_empty()
        && let Some(class_attr) = attribute_named_mut(element.attributes.as_mut_slice(), "class")
    {
        let kept_classes = class_attr
            .value
            .split_ascii_whitespace()
            .filter(|class_name| {
                !selector_classes.iter().any(|selector_class| selector_class == class_name)
                    || remaining_selectors
                        .iter()
                        .any(|selector| selector.contains(format!(".{class_name}").as_str()))
            })
            .collect::<Vec<_>>();
        class_attr.value = kept_classes.join(" ");
        if class_attr.value.is_empty() {
            element.attributes.retain(|attribute| attribute.name != "class");
        }
    }
}

fn collect_remaining_selectors(stylesheets: &HashMap<NodeId, Vec<CssRule>>) -> Vec<String> {
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

fn json_bool(params: Option<&Value>, name: &str, default: bool) -> bool {
    params
        .and_then(|value| value.get(name))
        .and_then(Value::as_bool)
        .unwrap_or(default)
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

fn compat_find_references(attribute_name: &str, value: &str) -> HashSet<String> {
    let mut references = HashSet::new();
    collect_references_from_value(value, &mut references);
    if attribute_name == "begin" {
        collect_begin_references(value, &mut references);
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
            let ch = char::from(bytes[index]);
            if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':') {
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
    node_id: NodeId,
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
        rewritten = rewrite_begin_reference_value(rewritten.as_str(), old_id, new_id);
    }
    rewritten
}

fn rewrite_begin_reference_value(value: &str, old_id: &str, new_id: &str) -> String {
    let mut rewritten = String::with_capacity(value.len());
    let mut rest = value;
    let mut replaced = false;
    loop {
        let (segment, tail) = match rest.split_once(';') {
            Some((segment, tail)) => (segment, Some(tail)),
            None => (rest, None),
        };
        if replaced {
            rewritten.push_str(segment);
        } else {
            let rewritten_segment = rewrite_begin_reference_segment(segment, old_id, new_id);
            if rewritten_segment != segment {
                replaced = true;
            }
            rewritten.push_str(rewritten_segment.as_str());
        }
        let Some(tail) = tail else {
            break;
        };
        rewritten.push(';');
        rest = tail;
    }
    rewritten
}

fn rewrite_begin_reference_segment(segment: &str, old_id: &str, new_id: &str) -> String {
    let trimmed = segment.trim();
    let rewritten_trimmed = trimmed
        .strip_prefix(format!("{old_id}.").as_str())
        .map_or_else(|| trimmed.to_string(), |tail| format!("{new_id}.{tail}"));

    let leading_len = segment.len() - segment.trim_start().len();
    let trailing_len = segment.len() - segment.trim_end().len();
    let prefix = &segment[..leading_len];
    let suffix = &segment[segment.len() - trailing_len..];
    format!("{prefix}{rewritten_trimmed}{suffix}")
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

fn walk_element_ids(doc: &Document) -> Vec<NodeId> {
    doc.nodes
        .iter()
        .enumerate()
        .skip(1)
        .filter_map(|(node_id, node)| matches!(node.kind, NodeKind::Element(_)).then_some(node_id))
        .collect()
}

fn query_selector_all(doc: &Document, selector: &str) -> Vec<NodeId> {
    walk_element_ids(doc)
        .into_iter()
        .filter(|node_id| selector_matches(doc, *node_id, selector))
        .collect()
}

fn document_has_style_or_scripts(doc: &Document) -> bool {
    walk_element_ids(doc).into_iter().any(|node_id| {
        let Some(element) = node_element(doc, node_id) else {
            return false;
        };
        (element.name == "style" && doc.children(node_id).next().is_some())
            || element_has_scripts(doc, node_id)
    })
}

fn svg_has_defs_only(doc: &Document) -> bool {
    let root_children: Vec<_> = doc.children(doc.root_id()).collect();
    !root_children.is_empty()
        && root_children
            .iter()
            .all(|child_id| node_element_name(doc, *child_id) == Some("defs"))
}

fn is_in_foreign_object_subtree(doc: &Document, node_id: NodeId) -> bool {
    let mut current = doc.node(node_id).parent;
    while let Some(parent_id) = current {
        if node_element_name(doc, parent_id) == Some("foreignObject") {
            return true;
        }
        current = doc.node(parent_id).parent;
    }
    false
}

fn detach_node(doc: &mut Document, node_id: NodeId) {
    let Some(parent_id) = doc.node(node_id).parent else {
        return;
    };
    let retained_children = doc
        .children(parent_id)
        .filter(|child_id| *child_id != node_id)
        .collect::<Vec<_>>();
    doc.reorder_children(parent_id, &retained_children);
    let node = doc.node_mut(node_id);
    node.parent = None;
    node.next_sibling = None;
}

fn replace_children_with_text(doc: &mut Document, node_id: NodeId, text: &str) {
    let existing_children = doc.children(node_id).collect::<Vec<_>>();
    for child_id in existing_children {
        detach_node(doc, child_id);
    }
    doc.append_child(node_id, NodeKind::Text(text.to_string()));
}

fn collect_style_css(doc: &Document, node_id: NodeId) -> String {
    let mut css = String::new();
    for child_id in doc.children(node_id) {
        match &doc.node(child_id).kind {
            NodeKind::Text(text) | NodeKind::Cdata(text) => css.push_str(text),
            _ => {}
        }
    }
    css
}

fn is_presentation_attr(name: &str) -> bool {
    matches!(
        name,
        "alignment-baseline"
            | "baseline-shift"
            | "clip"
            | "clip-path"
            | "clip-rule"
            | "color"
            | "color-interpolation"
            | "color-interpolation-filters"
            | "color-profile"
            | "color-rendering"
            | "cursor"
            | "direction"
            | "display"
            | "dominant-baseline"
            | "fill"
            | "fill-opacity"
            | "fill-rule"
            | "filter"
            | "flood-color"
            | "flood-opacity"
            | "font-family"
            | "font-size"
            | "font-size-adjust"
            | "font-stretch"
            | "font-style"
            | "font-variant"
            | "font-weight"
            | "glyph-orientation-horizontal"
            | "glyph-orientation-vertical"
            | "image-rendering"
            | "kerning"
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
            | "stroke"
            | "stroke-dasharray"
            | "stroke-dashoffset"
            | "stroke-linecap"
            | "stroke-linejoin"
            | "stroke-miterlimit"
            | "stroke-opacity"
            | "stroke-width"
            | "text-anchor"
            | "text-decoration"
            | "text-rendering"
            | "transform"
            | "unicode-bidi"
            | "visibility"
            | "word-spacing"
            | "writing-mode"
    )
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
                continue;
            }
        }
        index += 1;
    }
    tokens
}

fn set_or_push_attribute(
    attributes: &mut Vec<Attribute>,
    name: &str,
    value: &str,
    quote: QuoteStyle,
) {
    if let Some(attribute) = attributes.iter_mut().find(|attribute| attribute.name == name) {
        attribute.value = value.to_string();
        attribute.quote = quote;
    } else {
        attributes.push(Attribute {
            name: name.to_string(),
            value: value.to_string(),
            quote,
        });
    }
}

fn attribute_named<'a>(attributes: &'a [Attribute], name: &str) -> Option<&'a Attribute> {
    attributes.iter().find(|attribute| attribute.name == name)
}

fn attribute_named_mut<'a>(
    attributes: &'a mut [Attribute],
    name: &str,
) -> Option<&'a mut Attribute> {
    attributes.iter_mut().find(|attribute| attribute.name == name)
}

fn attribute_value<'a>(attributes: &'a [Attribute], name: &str) -> Option<&'a str> {
    attribute_named(attributes, name).map(|attribute| attribute.value.as_str())
}

fn node_element(doc: &Document, node_id: NodeId) -> Option<&crate::ast::Element> {
    match &doc.node(node_id).kind {
        NodeKind::Element(element) => Some(element),
        _ => None,
    }
}

fn node_element_name(doc: &Document, node_id: NodeId) -> Option<&str> {
    node_element(doc, node_id).map(|element| element.name.as_str())
}

fn element_has_scripts(doc: &Document, node_id: NodeId) -> bool {
    if node_element_name(doc, node_id) == Some("script") {
        return true;
    }
    doc.children(node_id)
        .any(|child_id| element_has_scripts(doc, child_id))
}

#[cfg(test)]
mod tests {
    use super::{DirectPortSpikeConfig, optimize_with_direct_port_spike};
    use crate::{Config, PluginSpec, optimize};

    #[test]
    fn direct_port_spike_cleanupids_matches_baseline_on_simple_reference() {
        let svg = r#"<svg><defs><linearGradient id="paint"/><linearGradient id="paint"/></defs><rect fill="url(#paint)"/></svg>"#;
        let config = Config {
            plugins: vec![PluginSpec::Name("cleanupIds".to_string())],
            ..Config::default()
        };

        let baseline = optimize(svg, &config).expect("baseline");
        let spike =
            optimize_with_direct_port_spike(svg, &config, &DirectPortSpikeConfig::default()).expect("spike");

        assert_eq!(baseline.data, spike.data);
    }

    #[test]
    #[allow(clippy::literal_string_with_formatting_args)]
    fn direct_port_spike_runs_inline_styles_pipeline() {
        let svg = "<svg><style>.hero{fill:red}</style><path class=\"hero\"/></svg>";
        let config = Config {
            plugins: vec![PluginSpec::Name("inlineStyles".to_string())],
            ..Config::default()
        };

        let spike =
            optimize_with_direct_port_spike(svg, &config, &DirectPortSpikeConfig::default()).expect("spike");

        assert!(spike.data.contains("fill=\"red\"") || spike.data.contains("style=\"fill:red\""));
    }
}
