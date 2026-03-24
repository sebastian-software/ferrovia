use std::cmp::Ordering;

use crate::error::Result;
use crate::types::{XastCdata, XastChild, XastElement, XastRoot, XastText};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum StyleContentType {
    #[default]
    Text,
    Cdata,
}

#[derive(Debug, Clone, Default)]
struct State {
    first_style_path: Option<Vec<usize>>,
    first_style_had_media: bool,
    collected_styles: String,
    style_content_type: StyleContentType,
    remove_paths: Vec<Vec<usize>>,
    merged_into_first: bool,
}

/// Direct port of SVGO's `mergeStyles`.
///
/// # Errors
///
/// This direct port currently performs in-memory tree rewrites only and does
/// not produce operational errors.
pub fn apply(root: &mut XastRoot) -> Result<()> {
    let mut state = State::default();
    collect_styles(&root.children, &mut state, &mut Vec::new(), false);

    if let Some(first_path) = &state.first_style_path {
        if state.first_style_had_media
            && let Some(first_style) = get_element_mut_by_path(root, first_path)
        {
            first_style.remove_attribute("media");
        }
        if state.merged_into_first
            && let Some(first_style) = get_element_mut_by_path(root, first_path)
        {
            first_style.children = vec![match state.style_content_type {
                StyleContentType::Text => XastChild::Text(XastText {
                    value: state.collected_styles.clone(),
                }),
                StyleContentType::Cdata => XastChild::Cdata(XastCdata {
                    value: state.collected_styles.clone(),
                }),
            }];
        }
    }

    state.remove_paths.sort_by(compare_paths_desc);
    state.remove_paths.dedup();
    for path in state.remove_paths {
        remove_node_by_path(root, &path);
    }

    Ok(())
}

fn collect_styles(
    children: &[XastChild],
    state: &mut State,
    path: &mut Vec<usize>,
    inside_foreign_object: bool,
) {
    if inside_foreign_object {
        return;
    }

    for (index, child) in children.iter().enumerate() {
        let XastChild::Element(node) = child else {
            continue;
        };

        path.push(index);
        if node.name == "foreignObject" {
            path.pop();
            continue;
        }

        if node.name == "style" {
            collect_style_element(node, state, path);
        }

        collect_styles(&node.children, state, path, false);
        path.pop();
    }
}

fn collect_style_element(node: &XastElement, state: &mut State, path: &[usize]) {
    if let Some(style_type) = node.get_attribute("type")
        && !style_type.is_empty()
        && style_type != "text/css"
    {
        return;
    }

    let mut css = String::new();
    for child in &node.children {
        match child {
            XastChild::Text(text) => css.push_str(text.value.as_str()),
            XastChild::Cdata(cdata) => {
                state.style_content_type = StyleContentType::Cdata;
                css.push_str(cdata.value.as_str());
            }
            _ => {}
        }
    }

    if css.trim().is_empty() {
        state.remove_paths.push(path.to_vec());
        return;
    }

    let had_media = node.get_attribute("media").is_some();
    if let Some(media) = node.get_attribute("media") {
        state.collected_styles.push_str("@media ");
        state.collected_styles.push_str(media);
        state.collected_styles.push('{');
        state.collected_styles.push_str(css.as_str());
        state.collected_styles.push('}');
    } else {
        state.collected_styles.push_str(css.as_str());
    }

    if state.first_style_path.is_none() {
        state.first_style_path = Some(path.to_vec());
        state.first_style_had_media = had_media;
    } else {
        state.merged_into_first = true;
        state.remove_paths.push(path.to_vec());
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
    if path.is_empty() {
        return false;
    }

    descend(&mut root.children, path)
}

fn descend(children: &mut Vec<XastChild>, path: &[usize]) -> bool {
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
        descend(&mut element.children, tail)
}

fn compare_paths_desc(left: &Vec<usize>, right: &Vec<usize>) -> Ordering {
    right.cmp(left)
}
