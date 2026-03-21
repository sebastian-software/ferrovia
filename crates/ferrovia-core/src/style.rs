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
        (None, Some(_) | None) => {}
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

#[cfg(test)]
mod tests {
    use crate::parser::parse;

    use super::{parse_style_declarations, parse_stylesheet_rules, selector_matches};

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
}
