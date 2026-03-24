use ferrovia_csso_compat::{MinifyOptions, Usage, minify, minify_block};
use serde_json::Value;

use crate::error::Result;
use crate::svgo::tools::has_scripts;
use crate::types::{XastChild, XastElement, XastRoot};

#[derive(Debug, Clone)]
struct StyleElementRef {
    path: Vec<usize>,
}

#[derive(Debug, Clone)]
struct StyleAttrRef {
    path: Vec<usize>,
}

/// Direct port slice of SVGO's `minifyStyles`.
///
/// # Errors
///
/// This direct port currently performs in-memory tree rewrites only and does
/// not produce operational errors.
pub fn apply(root: &mut XastRoot, params: Option<&Value>) -> Result<()> {
    let usage_value = params.and_then(|value| value.get("usage"));
    let (enable_tags_usage, enable_ids_usage, enable_classes_usage, force_usage_deoptimized) =
        parse_usage_options(usage_value);

    let mut style_elements = Vec::<StyleElementRef>::new();
    let mut style_attributes = Vec::<StyleAttrRef>::new();
    let mut tags_usage = Vec::<String>::new();
    let mut ids_usage = Vec::<String>::new();
    let mut classes_usage = Vec::<String>::new();
    let mut deoptimized = false;

    collect_nodes(
        root,
        &mut Vec::new(),
        &mut style_elements,
        &mut style_attributes,
        &mut tags_usage,
        &mut ids_usage,
        &mut classes_usage,
        &mut deoptimized,
    );

    let mut usage = Usage::default();
    if !deoptimized || force_usage_deoptimized {
        if enable_tags_usage {
            usage.tags = tags_usage;
        }
        if enable_ids_usage {
            usage.ids = ids_usage;
        }
        if enable_classes_usage {
            usage.classes = classes_usage;
        }
    }
    let minify_options = MinifyOptions { usage: Some(usage) };

    let mut remove_style_paths = Vec::<Vec<usize>>::new();
    for style_ref in &style_elements {
        if let Some(style_node) = get_element_mut_by_path(root, &style_ref.path)
            && let Some(first_child) = style_node.children.first_mut()
        {
            let css_text = match first_child {
                XastChild::Text(text) => text.value.clone(),
                XastChild::Cdata(cdata) => cdata.value.clone(),
                _ => continue,
            };
            let minified = minify(css_text.as_str(), &minify_options).css;
            if minified.is_empty() {
                remove_style_paths.push(style_ref.path.clone());
                continue;
            }
            match first_child {
                XastChild::Text(text) => {
                    if css_text.contains('<') || css_text.contains('>') {
                        *first_child = XastChild::Cdata(crate::types::XastCdata { value: minified });
                    } else {
                        text.value = minified;
                    }
                }
                XastChild::Cdata(cdata) => {
                    cdata.value = minified;
                }
                _ => {}
            }
        }
    }

    for attr_ref in &style_attributes {
        if let Some(node) = get_element_mut_by_path(root, &attr_ref.path)
            && let Some(style) = node.get_attribute("style")
        {
            let minified = minify_block(style, &minify_options).css;
            node.set_attribute("style", minified);
        }
    }

    remove_style_paths.sort_by(|left, right| right.cmp(left));
    for path in remove_style_paths {
        remove_node_by_path(root, &path);
    }

    Ok(())
}

fn parse_usage_options(usage: Option<&Value>) -> (bool, bool, bool, bool) {
    let Some(usage) = usage else {
        return (true, true, true, false);
    };
    if let Some(enabled) = usage.as_bool() {
        return (enabled, enabled, enabled, false);
    }
    let enable_tags = usage
        .get("tags")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let enable_ids = usage
        .get("ids")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let enable_classes = usage
        .get("classes")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let force = usage
        .get("force")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    (enable_tags, enable_ids, enable_classes, force)
}

#[allow(clippy::too_many_arguments)]
fn collect_nodes(
    root: &XastRoot,
    path: &mut Vec<usize>,
    style_elements: &mut Vec<StyleElementRef>,
    style_attributes: &mut Vec<StyleAttrRef>,
    tags_usage: &mut Vec<String>,
    ids_usage: &mut Vec<String>,
    classes_usage: &mut Vec<String>,
    deoptimized: &mut bool,
) {
    collect_nodes_in_children(
        &root.children,
        path,
        style_elements,
        style_attributes,
        tags_usage,
        ids_usage,
        classes_usage,
        deoptimized,
    );
}

#[allow(clippy::too_many_arguments)]
fn collect_nodes_in_children(
    children: &[XastChild],
    path: &mut Vec<usize>,
    style_elements: &mut Vec<StyleElementRef>,
    style_attributes: &mut Vec<StyleAttrRef>,
    tags_usage: &mut Vec<String>,
    ids_usage: &mut Vec<String>,
    classes_usage: &mut Vec<String>,
    deoptimized: &mut bool,
) {
    for (index, child) in children.iter().enumerate() {
        let XastChild::Element(node) = child else {
            continue;
        };
        path.push(index);

        if has_scripts(node) {
            *deoptimized = true;
        }

        tags_usage.push(node.name.clone());
        if let Some(id) = node.get_attribute("id") {
            ids_usage.push(id.to_string());
        }
        if let Some(classes) = node.get_attribute("class") {
            classes_usage.extend(classes.split_ascii_whitespace().map(ToOwned::to_owned));
        }

        if node.name == "style" && !node.children.is_empty() {
            style_elements.push(StyleElementRef { path: path.clone() });
        } else if node.get_attribute("style").is_some() {
            style_attributes.push(StyleAttrRef { path: path.clone() });
        }

        collect_nodes_in_children(
            &node.children,
            path,
            style_elements,
            style_attributes,
            tags_usage,
            ids_usage,
            classes_usage,
            deoptimized,
        );
        path.pop();
    }
}

fn get_element_mut_by_path<'a>(root: &'a mut XastRoot, path: &[usize]) -> Option<&'a mut XastElement> {
    fn descend<'a>(children: &'a mut [XastChild], path: &[usize]) -> Option<&'a mut XastElement> {
        let (index, tail) = path.split_first()?;
        let child = children.get_mut(*index)?;
        let XastChild::Element(element) = child else {
            return None;
        };
        if tail.is_empty() {
            Some(element)
        } else {
            descend(&mut element.children, tail)
        }
    }

    descend(&mut root.children, path)
}

fn remove_node_by_path(root: &mut XastRoot, path: &[usize]) -> bool {
    descend_remove(&mut root.children, path)
}

fn descend_remove(children: &mut Vec<XastChild>, path: &[usize]) -> bool {
    let Some((&index, tail)) = path.split_first() else {
        return false;
    };
    if tail.is_empty() {
        if index >= children.len() {
            return false;
        }
        children.remove(index);
        return true;
    }
    let Some(XastChild::Element(element)) = children.get_mut(index) else {
        return false;
    };
    descend_remove(&mut element.children, tail)
}

