use crate::plugins::_collections::is_presentation_attr;
use crate::types::{
    ComputedStyle, Specificity, Stylesheet, StylesheetDeclaration, StylesheetRule, XastChild,
    XastElement, XastRoot,
};

#[must_use]
pub const fn compare_specificity(left: Specificity, right: Specificity) -> i32 {
    let mut index = 0usize;
    while index < 4 {
        if left[index] < right[index] {
            return -1;
        }
        if left[index] > right[index] {
            return 1;
        }
        index += 1;
    }
    0
}

#[must_use]
pub fn collect_stylesheet(root: &XastRoot) -> Stylesheet {
    let mut rules = Vec::new();
    collect_rules(&root.children, &mut rules);
    rules.sort_by(|left, right| compare_specificity(left.specificity, right.specificity).cmp(&0));
    Stylesheet { rules }
}

#[must_use]
pub fn compute_style(stylesheet: &Stylesheet, node: &XastElement) -> Vec<(String, ComputedStyle)> {
    let mut computed = Vec::<(String, ComputedStyle)>::new();

    for rule in &stylesheet.rules {
        if selector_matches(node, rule.selector.as_str()) {
            for declaration in &rule.declarations {
                upsert_computed_style(
                    &mut computed,
                    declaration.name.as_str(),
                    ComputedStyle::Static {
                        inherited: false,
                        value: declaration.value.clone(),
                    },
                );
            }
        }
    }

    if let Some(style) = node.get_attribute("style") {
        for declaration in parse_style_declarations(style) {
            upsert_computed_style(
                &mut computed,
                declaration.name.as_str(),
                ComputedStyle::Static {
                    inherited: false,
                    value: declaration.value,
                },
            );
        }
    }

    for attribute in &node.attributes {
        if is_presentation_attr(attribute.name.as_str()) || attribute.name == "transform" {
            upsert_computed_style(
                &mut computed,
                attribute.name.as_str(),
                ComputedStyle::Static {
                    inherited: false,
                    value: attribute.value.clone(),
                },
            );
        }
    }

    computed
}

#[must_use]
pub fn parse_style_declarations(css: &str) -> Vec<StylesheetDeclaration> {
    css.split(';')
        .filter_map(|part| {
            let (name, value) = part.split_once(':')?;
            let name = name.trim();
            let value = value.trim();
            if name.is_empty() || value.is_empty() {
                return None;
            }
            Some(StylesheetDeclaration {
                name: name.to_string(),
                value: value.to_string(),
                important: false,
            })
        })
        .collect()
}

fn collect_rules(children: &[XastChild], rules: &mut Vec<StylesheetRule>) {
    for child in children {
        if let XastChild::Element(element) = child {
            if element.name == "style" {
                for style_child in &element.children {
                    match style_child {
                        XastChild::Text(text) => {
                            rules.extend(parse_stylesheet_rules(text.value.as_str()));
                        }
                        XastChild::Cdata(cdata) => {
                            rules.extend(parse_stylesheet_rules(cdata.value.as_str()));
                        }
                        _ => {}
                    }
                }
            }
            collect_rules(&element.children, rules);
        }
    }
}

fn parse_stylesheet_rules(css: &str) -> Vec<StylesheetRule> {
    css.split('}')
        .filter_map(|part| {
            let (selector, block) = part.split_once('{')?;
            let selector = selector.trim();
            if selector.is_empty() {
                return None;
            }
            let declarations = parse_style_declarations(block)
                .into_iter()
                .filter(|declaration| is_presentation_attr(declaration.name.as_str()))
                .collect::<Vec<_>>();
            Some(StylesheetRule {
                selector: selector.to_string(),
                dynamic: false,
                specificity: [0, 0, 0, 0],
                declarations,
            })
        })
        .collect()
}

fn upsert_computed_style(
    computed: &mut Vec<(String, ComputedStyle)>,
    name: &str,
    value: ComputedStyle,
) {
    if let Some((_, existing)) = computed
        .iter_mut()
        .find(|(existing_name, _)| existing_name == name)
    {
        *existing = value;
        return;
    }
    computed.push((name.to_string(), value));
}

fn selector_matches(node: &XastElement, selector: &str) -> bool {
    let selector = selector.trim();
    if selector.is_empty() {
        return false;
    }
    if let Some(id) = selector.strip_prefix('#') {
        return node.get_attribute("id").is_some_and(|value| value == id);
    }
    if let Some(class_name) = selector.strip_prefix('.') {
        return node.get_attribute("class").is_some_and(|value| {
            value
                .split_ascii_whitespace()
                .any(|item| item == class_name)
        });
    }
    node.name == selector
}
