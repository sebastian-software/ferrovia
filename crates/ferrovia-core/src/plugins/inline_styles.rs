use serde_json::Value;

use crate::error::Result;
use crate::plugins::_collections::is_presentation_attr;
use crate::style::{compare_specificity, includes_attr_selector, parse_style_declarations};
use crate::types::{Specificity, StylesheetDeclaration, XastChild, XastElement, XastRoot};
use crate::xast::query_selector_all;

#[derive(Debug, Clone)]
struct StyleNodeRef {
    path: Vec<usize>,
}

#[derive(Debug, Clone)]
struct SelectorRef {
    style_index: usize,
    rule_index: usize,
    selector_index: usize,
    selector_text: String,
    specificity: Specificity,
}

#[derive(Debug, Clone)]
struct StyleSheet {
    content_type: StyleContentType,
    rules: Vec<StyleRule>,
}

#[derive(Debug, Clone)]
struct StyleRule {
    selectors: Vec<String>,
    declarations: Vec<StylesheetDeclaration>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum StyleContentType {
    #[default]
    Text,
    Cdata,
}

/// Direct port slice of SVGO's `inlineStyles`.
///
/// # Errors
///
/// This direct port currently performs in-memory tree rewrites only and does
/// not produce operational errors.
#[allow(clippy::too_many_lines)]
pub fn apply(root: &mut XastRoot, params: Option<&Value>) -> Result<()> {
    let only_matched_once = params
        .and_then(|value| value.get("onlyMatchedOnce"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let remove_matched_selectors = params
        .and_then(|value| value.get("removeMatchedSelectors"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let use_mqs = params
        .and_then(|value| value.get("useMqs"))
        .and_then(Value::as_array)
        .map_or_else(
            || vec![String::new(), "screen".to_string()],
            |items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>()
            },
        );

    let mut styles = Vec::<StyleNodeRef>::new();
    let mut parsed = Vec::<StyleSheet>::new();
    collect_style_nodes(root, &mut Vec::new(), false, &use_mqs, &mut styles, &mut parsed);
    if styles.is_empty() {
        return Ok(());
    }

    let mut selectors = collect_selectors(&parsed);
    selectors.sort_by(|left, right| compare_specificity(left.specificity, right.specificity).cmp(&0));
    selectors.reverse();

    let mut matched_sets = Vec::<Vec<Vec<usize>>>::new();
    for selector in &selectors {
        let matched_paths = collect_match_paths(root, selector.selector_text.as_str());
        if matched_paths.is_empty() {
            matched_sets.push(Vec::new());
            continue;
        }
        if only_matched_once && matched_paths.len() > 1 {
            matched_sets.push(matched_paths);
            continue;
        }

        let selector_texts = current_selector_texts(&parsed);
        let rule_declarations =
            parsed[selector.style_index].rules[selector.rule_index].declarations.clone();

        for path in &matched_paths {
            if let Some(element) = get_element_mut_by_path(root, path) {
                apply_rule_to_element(element, &rule_declarations, &selector_texts);
            }
        }

        if remove_matched_selectors
            && let Some(rule) = parsed
                .get_mut(selector.style_index)
                .and_then(|style| style.rules.get_mut(selector.rule_index))
            && selector.selector_index < rule.selectors.len()
        {
            rule.selectors.remove(selector.selector_index);
        }

        matched_sets.push(matched_paths);
    }

    if remove_matched_selectors {
        let active_selectors = current_selector_texts(&parsed);
        for (selector, matched_paths) in selectors.iter().zip(&matched_sets) {
            if matched_paths.is_empty() {
                continue;
            }
            if only_matched_once && matched_paths.len() > 1 {
                continue;
            }
            for path in matched_paths {
                if let Some(element) = get_element_mut_by_path(root, path) {
                    cleanup_selector_attributes(
                        element,
                        selector.selector_text.as_str(),
                        &active_selectors,
                    );
                }
            }
        }
    }

    let mut remove_style_paths = Vec::<Vec<usize>>::new();
    for (index, style_ref) in styles.iter().enumerate() {
        parsed[index].rules.retain(|rule| !rule.selectors.is_empty());
        if parsed[index].rules.is_empty() {
            remove_style_paths.push(style_ref.path.clone());
            continue;
        }
        let css = generate_stylesheet(&parsed[index]);
        if let Some(style) = get_element_mut_by_path(root, &style_ref.path)
            && let Some(first_child) = style
                .children
                .iter_mut()
                .find(|child| matches!(child, XastChild::Text(_) | XastChild::Cdata(_)))
        {
            match first_child {
                XastChild::Text(text) => text.value.clone_from(&css),
                XastChild::Cdata(cdata) => cdata.value.clone_from(&css),
                _ => {}
            }
        }
    }

    remove_style_paths.sort_by(|left, right| right.cmp(left));
    for path in remove_style_paths {
        remove_node_by_path(root, &path);
    }

    Ok(())
}

fn collect_style_nodes(
    root: &XastRoot,
    path: &mut Vec<usize>,
    inside_foreign_object: bool,
    use_mqs: &[String],
    styles: &mut Vec<StyleNodeRef>,
    parsed: &mut Vec<StyleSheet>,
) {
    collect_style_nodes_in_children(
        &root.children,
        path,
        inside_foreign_object,
        use_mqs,
        styles,
        parsed,
    );
}

fn collect_style_nodes_in_children(
    children: &[XastChild],
    path: &mut Vec<usize>,
    inside_foreign_object: bool,
    use_mqs: &[String],
    styles: &mut Vec<StyleNodeRef>,
    parsed: &mut Vec<StyleSheet>,
) {
    if inside_foreign_object {
        return;
    }

    for (index, child) in children.iter().enumerate() {
        let XastChild::Element(element) = child else {
            continue;
        };
        path.push(index);
        if element.name == "foreignObject" {
            path.pop();
            continue;
        }
        if element.name == "style"
            && !element.children.is_empty()
            && element
                .get_attribute("type")
                .is_none_or(|style_type| style_type.is_empty() || style_type == "text/css")
        {
            let stylesheet = parse_style_element(element, use_mqs);
            styles.push(StyleNodeRef { path: path.clone() });
            parsed.push(stylesheet);
        }
        collect_style_nodes_in_children(
            &element.children,
            path,
            false,
            use_mqs,
            styles,
            parsed,
        );
        path.pop();
    }
}

fn parse_style_element(node: &XastElement, use_mqs: &[String]) -> StyleSheet {
    let mut css_text = String::new();
    let mut content_type = StyleContentType::Text;
    for child in &node.children {
        match child {
            XastChild::Text(text) => css_text.push_str(text.value.as_str()),
            XastChild::Cdata(cdata) => {
                content_type = StyleContentType::Cdata;
                css_text.push_str(cdata.value.as_str());
            }
            _ => {}
        }
    }

    let rules = parse_stylesheet_rules(css_text.as_str(), use_mqs);
    StyleSheet {
        content_type,
        rules,
    }
}

fn parse_stylesheet_rules(css: &str, use_mqs: &[String]) -> Vec<StyleRule> {
    let mut rules = Vec::<StyleRule>::new();
    let mut index = 0usize;
    let bytes = css.as_bytes();
    while index < bytes.len() {
        skip_ascii_whitespace(bytes, &mut index);
        if index >= bytes.len() {
            break;
        }
        if css[index..].starts_with("@media") {
            index += "@media".len();
            let start = index;
            while index < bytes.len() && bytes[index] != b'{' {
                index += 1;
            }
            let media = css[start..index].trim();
            if index >= bytes.len() {
                break;
            }
            let (block, next_index) = take_block(css, index);
            if use_mqs.iter().any(|query| query == &format!("media {media}") || query == &format!("@media {media}") || query == media) {
                rules.extend(parse_stylesheet_rules(block.as_str(), &[String::new()]));
            }
            index = next_index;
            continue;
        }

        let selector_start = index;
        while index < bytes.len() && bytes[index] != b'{' {
            index += 1;
        }
        if index >= bytes.len() {
            break;
        }
        let selector_list = css[selector_start..index].trim();
        let (block, next_index) = take_block(css, index);
        let declarations = parse_style_declarations(block.as_str());
        if !selector_list.is_empty() && !declarations.is_empty() {
            let selectors = selector_list
                .split(',')
                .map(str::trim)
                .filter_map(normalize_selector)
                .collect::<Vec<_>>();
            if !selectors.is_empty() {
                rules.push(StyleRule {
                    selectors,
                    declarations,
                });
            }
        }
        index = next_index;
    }
    rules
}

fn take_block(css: &str, open_brace_index: usize) -> (String, usize) {
    let bytes = css.as_bytes();
    let mut depth = 0usize;
    let mut index = open_brace_index;
    let mut quote = None::<u8>;
    while index < bytes.len() {
        let byte = bytes[index];
        if let Some(current_quote) = quote {
            if byte == current_quote {
                quote = None;
            }
        } else {
            match byte {
                b'"' | b'\'' => quote = Some(byte),
                b'{' => depth += 1,
                b'}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        let content = css[open_brace_index + 1..index].to_string();
                        return (content, index + 1);
                    }
                }
                _ => {}
            }
        }
        index += 1;
    }
    (String::new(), bytes.len())
}

fn normalize_selector(selector: &str) -> Option<String> {
    let mut result = String::new();
    let chars = selector.chars().collect::<Vec<_>>();
    let mut index = 0usize;
    while index < chars.len() {
        if chars[index] == ':' {
            index += 1;
            if index < chars.len() && chars[index] == ':' {
                index += 1;
            }
            while index < chars.len()
                && (chars[index].is_ascii_alphanumeric() || chars[index] == '-' || chars[index] == '_')
            {
                index += 1;
            }
            if index < chars.len() && chars[index] == '(' {
                let mut depth = 0usize;
                while index < chars.len() {
                    let ch = chars[index];
                    if ch == '(' {
                        depth += 1;
                    } else if ch == ')' {
                        depth = depth.saturating_sub(1);
                        if depth == 0 {
                            index += 1;
                            break;
                        }
                    }
                    index += 1;
                }
            }
            continue;
        }
        result.push(chars[index]);
        index += 1;
    }
    let normalized = result.trim();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized.to_string())
    }
}

fn skip_ascii_whitespace(bytes: &[u8], index: &mut usize) {
    while *index < bytes.len() && bytes[*index].is_ascii_whitespace() {
        *index += 1;
    }
}

fn collect_selectors(stylesheets: &[StyleSheet]) -> Vec<SelectorRef> {
    let mut selectors = Vec::<SelectorRef>::new();
    for (style_index, stylesheet) in stylesheets.iter().enumerate() {
        for (rule_index, rule) in stylesheet.rules.iter().enumerate() {
            for (selector_index, selector_text) in rule.selectors.iter().enumerate() {
                selectors.push(SelectorRef {
                    style_index,
                    rule_index,
                    selector_index,
                    specificity: compute_specificity(selector_text.as_str()),
                    selector_text: selector_text.clone(),
                });
            }
        }
    }
    selectors
}

fn compute_specificity(selector: &str) -> Specificity {
    let groups = ferrovia_css_what_compat::parse(selector);
    let mut best = [0u32; 4];
    for group in groups {
        let mut specificity = [0u32; 4];
        for token in group.tokens {
            if token.compound.id.is_some() {
                specificity[1] += 1;
            }
            specificity[2] += narrow_u32(token.compound.classes.len());
            specificity[2] += narrow_u32(token.compound.attributes.len());
            if token.compound.tag.is_some() {
                specificity[3] += 1;
            }
        }
        if compare_specificity(specificity, best) == 1 {
            best = specificity;
        }
    }
    best
}

fn collect_match_paths(root: &XastRoot, selector: &str) -> Vec<Vec<usize>> {
    let matches = query_selector_all(root, selector);
    let mut paths = Vec::<Vec<usize>>::new();
    for matched in matches {
        if let Some(path) = find_path(&root.children, matched, &mut Vec::new()) {
            paths.push(path);
        }
    }
    paths
}

fn current_selector_texts(stylesheets: &[StyleSheet]) -> Vec<String> {
    stylesheets
        .iter()
        .flat_map(|style| style.rules.iter())
        .flat_map(|rule| rule.selectors.iter().cloned())
        .collect()
}

fn apply_rule_to_element(
    element: &mut XastElement,
    declarations: &[StylesheetDeclaration],
    selector_texts: &[String],
) {
    let mut inline_styles = parse_style_declarations(element.get_attribute("style").unwrap_or(""));
    for declaration in declarations {
        let property = declaration.name.as_str();
        if is_presentation_attr(property)
            && !selector_texts
                .iter()
                .any(|selector| includes_attr_selector(selector, property, None, false))
        {
            element.remove_attribute(property);
        }

        if let Some(existing) = inline_styles.iter_mut().find(|existing| existing.name == property) {
            if !existing.important || declaration.important {
                *existing = declaration.clone();
            }
        } else {
            inline_styles.push(declaration.clone());
        }
    }

    let new_style = inline_styles
        .iter()
        .map(|declaration| {
            let mut fragment = format!("{}:{}", declaration.name, declaration.value);
            if declaration.important {
                fragment.push_str("!important");
            }
            fragment
        })
        .collect::<Vec<_>>()
        .join(";");

    if new_style.is_empty() {
        element.remove_attribute("style");
    } else {
        element.set_attribute("style", new_style);
    }
}

fn cleanup_selector_attributes(
    element: &mut XastElement,
    selector: &str,
    active_selectors: &[String],
) {
    let groups = ferrovia_css_what_compat::parse(selector);
    for group in groups {
        for (index, token) in group.tokens.iter().enumerate() {
            let traversed = index != group.tokens.len().saturating_sub(1);
            for class_name in &token.compound.classes {
                if !active_selectors.iter().any(|active| {
                    includes_attr_selector(active, "class", Some(class_name.as_str()), traversed)
                }) {
                    remove_class_value(element, class_name);
                }
            }
            if let Some(id) = &token.compound.id
                && element.get_attribute("id") == Some(id.as_str())
                && !active_selectors.iter().any(|active| {
                    includes_attr_selector(active, "id", Some(id.as_str()), traversed)
                })
            {
                element.remove_attribute("id");
            }
        }
    }
}

fn remove_class_value(element: &mut XastElement, class_name: &str) {
    let Some(class_attr) = element.get_attribute("class") else {
        return;
    };
    let next = class_attr
        .split_ascii_whitespace()
        .filter(|item| *item != class_name)
        .collect::<Vec<_>>();
    if next.is_empty() {
        element.remove_attribute("class");
    } else {
        element.set_attribute("class", next.join(" "));
    }
}

fn generate_stylesheet(stylesheet: &StyleSheet) -> String {
    let _ = stylesheet.content_type;
    let mut css = String::new();
    for rule in &stylesheet.rules {
        css.push_str(rule.selectors.join(",").as_str());
        css.push('{');
        for (index, declaration) in rule.declarations.iter().enumerate() {
            if index > 0 {
                css.push(';');
            }
            css.push_str(declaration.name.as_str());
            css.push(':');
            css.push_str(declaration.value.as_str());
            if declaration.important {
                css.push_str("!important");
            }
        }
        css.push('}');
    }
    css
}

fn narrow_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn find_path(
    children: &[XastChild],
    target: &XastChild,
    prefix: &mut Vec<usize>,
) -> Option<Vec<usize>> {
    for (index, child) in children.iter().enumerate() {
        prefix.push(index);
        if std::ptr::eq(child, target) {
            return Some(prefix.clone());
        }
        if let XastChild::Element(element) = child
            && let Some(path) = find_path(&element.children, target, prefix)
        {
            return Some(path);
        }
        prefix.pop();
    }
    None
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
