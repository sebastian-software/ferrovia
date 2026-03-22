#![allow(dead_code)]

use simplecss::{AttributeOperator, DeclarationTokenizer, Element as SimpleCssElement, PseudoClass};

use crate::ast::{Attribute, Document, NodeId, NodeKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StyleDeclaration {
    pub(crate) name: String,
    pub(crate) value: String,
    pub(crate) important: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StylesheetRule {
    pub(crate) selector: String,
    pub(crate) specificity: [u8; 3],
    pub(crate) declarations: Vec<StyleDeclaration>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CssRule {
    pub(crate) selectors: Vec<String>,
    pub(crate) media_query: Option<String>,
    pub(crate) declarations: Vec<StyleDeclaration>,
}

pub(crate) fn parse_style_declarations(style: &str) -> Vec<StyleDeclaration> {
    DeclarationTokenizer::from(style)
        .filter(|declaration| {
            !declaration.name.trim().is_empty() && !declaration.value.trim().is_empty()
        })
        .map(|declaration| StyleDeclaration {
            name: declaration.name.trim().to_string(),
            value: declaration.value.trim().to_string(),
            important: declaration.important,
        })
        .collect()
}

pub(crate) fn parse_stylesheet_rules(css: &str) -> Vec<StylesheetRule> {
    simplecss::StyleSheet::parse(css)
        .rules
        .into_iter()
        .map(|rule| StylesheetRule {
            selector: rule.selector.to_string(),
            specificity: rule.selector.specificity(),
            declarations: rule
                .declarations
                .into_iter()
                .map(|declaration| StyleDeclaration {
                    name: declaration.name.trim().to_string(),
                    value: declaration.value.trim().to_string(),
                    important: declaration.important,
                })
                .collect(),
        })
        .collect()
}

pub(crate) fn parse_css_rules(css: &str) -> Vec<CssRule> {
    let css = strip_css_comments(css);
    let mut rules = Vec::new();
    let mut index = 0;
    while index < css.len() {
        skip_css_whitespace(&css, &mut index);
        if index >= css.len() {
            break;
        }
        let remainder = &css[index..];
        if remainder.starts_with("@media") {
            index += "@media".len();
            let media_start = index;
            while index < css.len() && css.as_bytes()[index] != b'{' {
                index += 1;
            }
            if index >= css.len() {
                break;
            }
            let media_query = css[media_start..index].trim();
            let Some((body, next_index)) = consume_css_block(&css, index) else {
                break;
            };
            rules.extend(parse_css_rules_in_media(body, media_query));
            index = next_index;
            continue;
        }
        if remainder.starts_with('@') {
            skip_css_at_rule(&css, &mut index);
            continue;
        }
        let selector_start = index;
        while index < css.len() && css.as_bytes()[index] != b'{' {
            index += 1;
        }
        if index >= css.len() {
            break;
        }
        let selector_text = css[selector_start..index].trim();
        let Some((body, next_index)) = consume_css_block(&css, index) else {
            break;
        };
        let declarations = parse_style_declarations(body);
        if !selector_text.is_empty() && !declarations.is_empty() {
            let selectors = split_css_selectors(selector_text);
            if !selectors.is_empty() {
                rules.push(CssRule {
                    selectors,
                    media_query: None,
                    declarations,
                });
            }
        }
        index = next_index;
    }
    rules
}

pub(crate) fn serialize_css_rules(rules: &[CssRule]) -> String {
    let mut serialized = String::new();
    let mut index = 0;
    while index < rules.len() {
        let current_media = rules[index].media_query.clone();
        if let Some(media_query) = current_media {
            serialized.push_str("@media ");
            serialized.push_str(media_query.as_str());
            serialized.push('{');
            while index < rules.len() && rules[index].media_query.as_deref() == Some(media_query.as_str()) {
                serialize_css_rule(&mut serialized, &rules[index]);
                index += 1;
            }
            serialized.push('}');
        } else {
            serialize_css_rule(&mut serialized, &rules[index]);
            index += 1;
        }
    }
    serialized
}

pub(crate) fn selector_specificity(selector: &str) -> Option<[u8; 3]> {
    Some(simplecss::Selector::parse(selector)?.specificity())
}

pub(crate) fn selector_matches(doc: &Document, node_id: NodeId, selector: &str) -> bool {
    let Some(parsed) = simplecss::Selector::parse(selector) else {
        return false;
    };
    parsed.matches(&SimpleCssNode { doc, node_id })
}

pub(crate) fn serialize_style_declarations(declarations: &[StyleDeclaration]) -> String {
    declarations
        .iter()
        .map(|declaration| {
            let mut serialized = format!("{}:{}", declaration.name, declaration.value);
            if declaration.important {
                serialized.push_str(" !important");
            }
            serialized
        })
        .collect::<Vec<_>>()
        .join(";")
}

pub(crate) fn serialize_minified_style_declarations(declarations: &[StyleDeclaration]) -> String {
    declarations
        .iter()
        .map(|declaration| {
            let mut serialized = format!("{}:{}", declaration.name, declaration.value);
            if declaration.important {
                serialized.push_str("!important");
            }
            serialized
        })
        .collect::<Vec<_>>()
        .join(";")
}

pub(crate) fn dedupe_declarations(declarations: &mut Vec<StyleDeclaration>, name: &str) {
    let mut keep_index = declarations
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, declaration)| (declaration.name == name).then_some(index));
    if keep_index.is_none() {
        return;
    }
    let keep_index = keep_index.take().unwrap_or_default();
    let mut index = 0;
    declarations.retain(|declaration| {
        let keep = declaration.name != name || index == keep_index;
        index += 1;
        keep
    });
}

pub(crate) fn remove_declarations(declarations: &mut Vec<StyleDeclaration>, name: &str) {
    declarations.retain(|declaration| declaration.name != name);
}

pub(crate) fn update_style_attribute(
    attributes: &mut Vec<Attribute>,
    declarations: &[StyleDeclaration],
) {
    let serialized = if declarations.is_empty() {
        None
    } else {
        Some(serialize_style_declarations(declarations))
    };
    let style_index = attributes
        .iter()
        .position(|attribute| attribute.name == "style");
    match (style_index, serialized) {
        (Some(index), Some(value)) => attributes[index].value = value,
        (Some(_), None) => attributes.retain(|attribute| attribute.name != "style"),
        (None, Some(value)) => attributes.push(Attribute {
            name: "style".to_string(),
            value,
            quote: crate::ast::QuoteStyle::Double,
        }),
        (None, None) => {}
    }
}

#[derive(Clone, Copy)]
struct SimpleCssNode<'a> {
    doc: &'a Document,
    node_id: NodeId,
}

impl SimpleCssNode<'_> {
    fn element(&self) -> Option<&crate::ast::Element> {
        match &self.doc.node(self.node_id).kind {
            NodeKind::Element(element) => Some(element),
            _ => None,
        }
    }
}

impl SimpleCssElement for SimpleCssNode<'_> {
    fn parent_element(&self) -> Option<Self> {
        let parent = self.doc.node(self.node_id).parent?;
        if matches!(self.doc.node(parent).kind, NodeKind::Element(_)) {
            Some(Self {
                doc: self.doc,
                node_id: parent,
            })
        } else {
            None
        }
    }

    fn prev_sibling_element(&self) -> Option<Self> {
        let parent = self.doc.node(self.node_id).parent?;
        let mut previous = None;
        for child_id in self.doc.children(parent) {
            if child_id == self.node_id {
                return previous;
            }
            if matches!(self.doc.node(child_id).kind, NodeKind::Element(_)) {
                previous = Some(Self {
                    doc: self.doc,
                    node_id: child_id,
                });
            }
        }
        None
    }

    fn has_local_name(&self, name: &str) -> bool {
        self.element()
            .is_some_and(|element| local_name(element.name.as_str()) == name)
    }

    fn attribute_matches(&self, name: &str, operator: AttributeOperator<'_>) -> bool {
        let Some(element) = self.element() else {
            return false;
        };
        element.attributes.iter().any(|attribute| {
            local_name(attribute.name.as_str()) == name && operator.matches(attribute.value.as_str())
        })
    }

    fn pseudo_class_matches(&self, class: PseudoClass<'_>) -> bool {
        match class {
            PseudoClass::FirstChild => self.prev_sibling_element().is_none(),
            PseudoClass::Lang(lang) => {
                let mut current = Some(*self);
                while let Some(node) = current {
                    let Some(element) = node.element() else {
                        break;
                    };
                    for attribute in &element.attributes {
                        if matches!(attribute.name.as_str(), "lang" | "xml:lang") {
                            return attribute.value == lang
                                || attribute
                                    .value
                                    .strip_prefix(lang)
                                    .is_some_and(|tail| tail.starts_with('-'));
                        }
                    }
                    current = node.parent_element();
                }
                false
            }
            PseudoClass::Link
            | PseudoClass::Visited
            | PseudoClass::Hover
            | PseudoClass::Active
            | PseudoClass::Focus => false,
        }
    }
}

fn local_name(name: &str) -> &str {
    name.rsplit_once(':').map_or(name, |(_, local)| local)
}

fn parse_css_rules_in_media(css: &str, media_query: &str) -> Vec<CssRule> {
    parse_css_rules(css)
        .into_iter()
        .map(|mut rule| {
            rule.media_query = Some(media_query.to_string());
            rule
        })
        .collect()
}

fn skip_css_whitespace(css: &str, index: &mut usize) {
    while *index < css.len() && css.as_bytes()[*index].is_ascii_whitespace() {
        *index += 1;
    }
}

fn skip_css_at_rule(css: &str, index: &mut usize) {
    while *index < css.len() && css.as_bytes()[*index] != b'{' && css.as_bytes()[*index] != b';' {
        *index += 1;
    }
    if *index >= css.len() {
        return;
    }
    if css.as_bytes()[*index] == b';' {
        *index += 1;
        return;
    }
    if let Some((_, next_index)) = consume_css_block(css, *index) {
        *index = next_index;
    } else {
        *index = css.len();
    }
}

fn consume_css_block(css: &str, open_brace_index: usize) -> Option<(&str, usize)> {
    let bytes = css.as_bytes();
    if bytes.get(open_brace_index) != Some(&b'{') {
        return None;
    }
    let mut index = open_brace_index + 1;
    let mut depth = 1usize;
    let mut quote = None;
    while index < bytes.len() {
        let byte = bytes[index];
        if let Some(delimiter) = quote {
            if byte == b'\\' {
                index += 2;
                continue;
            }
            if byte == delimiter {
                quote = None;
            }
            index += 1;
            continue;
        }
        match byte {
            b'"' | b'\'' => {
                quote = Some(byte);
            }
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((&css[open_brace_index + 1..index], index + 1));
                }
            }
            _ => {}
        }
        index += 1;
    }
    None
}

fn split_css_selectors(selector_text: &str) -> Vec<String> {
    let mut selectors = Vec::new();
    let mut current = String::new();
    let mut bracket_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut quote = None;
    for ch in selector_text.chars() {
        if let Some(delimiter) = quote {
            current.push(ch);
            if ch == delimiter {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' => {
                quote = Some(ch);
                current.push(ch);
            }
            '[' => {
                bracket_depth += 1;
                current.push(ch);
            }
            ']' => {
                bracket_depth = bracket_depth.saturating_sub(1);
                current.push(ch);
            }
            '(' => {
                paren_depth += 1;
                current.push(ch);
            }
            ')' => {
                paren_depth = paren_depth.saturating_sub(1);
                current.push(ch);
            }
            ',' if bracket_depth == 0 && paren_depth == 0 => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    selectors.push(trimmed.to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        selectors.push(trimmed.to_string());
    }
    selectors
}

fn serialize_css_rule(serialized: &mut String, rule: &CssRule) {
    if rule.selectors.is_empty() || rule.declarations.is_empty() {
        return;
    }
    serialized.push_str(rule.selectors.join(",").as_str());
    serialized.push('{');
    serialized.push_str(
        serialize_minified_style_declarations(&rule.declarations).as_str(),
    );
    serialized.push('}');
}

fn strip_css_comments(css: &str) -> String {
    let bytes = css.as_bytes();
    let mut out = String::with_capacity(css.len());
    let mut index = 0;
    let mut quote = None;
    while index < bytes.len() {
        let byte = bytes[index];
        if let Some(delimiter) = quote {
            out.push(char::from(byte));
            if byte == b'\\' {
                if let Some(next) = bytes.get(index + 1) {
                    out.push(char::from(*next));
                    index += 2;
                    continue;
                }
            } else if byte == delimiter {
                quote = None;
            }
            index += 1;
            continue;
        }
        if matches!(byte, b'"' | b'\'') {
            quote = Some(byte);
            out.push(char::from(byte));
            index += 1;
            continue;
        }
        if byte == b'/' && bytes.get(index + 1) == Some(&b'*') {
            index += 2;
            while index + 1 < bytes.len() && !(bytes[index] == b'*' && bytes[index + 1] == b'/') {
                index += 1;
            }
            index = (index + 2).min(bytes.len());
            continue;
        }
        out.push(char::from(byte));
        index += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use crate::parser::parse;

    use super::{
        parse_css_rules, parse_style_declarations, parse_stylesheet_rules, selector_matches,
        serialize_css_rules,
    };

    #[test]
    fn parses_inline_style_with_important_values() {
        let declarations = parse_style_declarations(
            "fill: red; stroke-width:2!important; opacity: 0.5",
        );
        assert_eq!(declarations.len(), 3);
        assert_eq!(declarations[0].name, "fill");
        assert_eq!(declarations[1].name, "stroke-width");
        assert!(declarations[1].important);
        assert_eq!(declarations[2].value, "0.5");
    }

    #[test]
    fn parses_stylesheet_rules_with_specificity() {
        let rules = parse_stylesheet_rules(
            "g.notice > path[stroke='red'] { fill:none; stroke-width:2 } #hero { opacity:1 }",
        );
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].selector, "g[class~='notice'] > path[stroke='red']");
        assert_eq!(rules[0].specificity, [0, 2, 2]);
        assert_eq!(rules[0].declarations[0].name, "fill");
        assert_eq!(rules[1].specificity, [1, 0, 0]);
    }

    #[test]
    fn matches_simplecss_selectors_against_ferrovia_tree() {
        let doc = parse(
            r#"<svg xmlns="http://www.w3.org/2000/svg"><g class="notice" xml:lang="en"><path id="hero" stroke="red"/><path class="secondary"/></g></svg>"#,
        )
        .expect("parse");
        let svg_id = doc.children(doc.root_id()).next().expect("svg");
        let g_id = doc.children(svg_id).next().expect("group");
        let path_ids: Vec<_> = doc.children(g_id).collect();

        assert!(selector_matches(&doc, path_ids[0], "g.notice > path[stroke='red']"));
        assert!(selector_matches(&doc, path_ids[0], "#hero"));
        assert!(selector_matches(&doc, path_ids[0], "g:lang(en) > path:first-child"));
        assert!(!selector_matches(&doc, path_ids[1], "g:lang(de) > path:first-child"));
    }

    #[test]
    fn parses_css_rules_with_media_queries() {
        let rules = parse_css_rules(
            "rect, path { fill: red; stroke-width: 2 } @media screen { #hero { opacity: 1 } }",
        );
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].selectors, vec!["rect".to_string(), "path".to_string()]);
        assert_eq!(rules[1].media_query.as_deref(), Some("screen"));
        assert_eq!(rules[1].selectors, vec!["#hero".to_string()]);
    }

    #[test]
    fn serializes_css_rules_compactly() {
        let css = "rect { fill: red } @media screen { #hero { opacity: 1 } }";
        let parsed = parse_css_rules(css);
        let expected = concat!("rect{fill:red}", "@media screen{#hero{opacity:1}}");
        assert_eq!(
            serialize_css_rules(&parsed),
            expected
        );
    }
}
