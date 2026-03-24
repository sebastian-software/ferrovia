use crate::types::{XastChild, XastElement, XastRoot};

pub fn detach_node_from_parent(children: &mut Vec<XastChild>, index: usize) {
    children.remove(index);
}

#[must_use]
pub fn query_selector_all<'a>(root: &'a XastRoot, selector: &str) -> Vec<&'a XastChild> {
    let selectors = parse_selectors(selector);
    if selectors.is_empty() {
        return Vec::new();
    }

    let mut results = Vec::<&'a XastChild>::new();
    let mut ancestry = Vec::<&'a XastElement>::new();
    collect_matches(&root.children, &selectors, &mut ancestry, &mut results);
    results
}

#[must_use]
pub fn query_selector<'a>(root: &'a XastRoot, selector: &str) -> Option<&'a XastChild> {
    query_selector_all(root, selector).into_iter().next()
}

#[must_use]
pub fn matches(node: &XastElement, selector: &str) -> bool {
    let selectors = parse_selectors(selector);
    selectors.iter().any(|parsed| {
        parsed.parts.len() == 1
            && parsed
                .parts
                .last()
                .is_some_and(|part| matches_compound(node, &part.compound))
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Combinator {
    Descendant,
    Child,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedSelector {
    parts: Vec<SelectorPart>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SelectorPart {
    combinator: Option<Combinator>,
    compound: CompoundSelector,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct CompoundSelector {
    tag: Option<String>,
    id: Option<String>,
    classes: Vec<String>,
    attributes: Vec<AttributeSelector>,
    universal: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AttributeSelector {
    name: String,
    value: Option<String>,
}

fn collect_matches<'a>(
    children: &'a [XastChild],
    selectors: &[ParsedSelector],
    ancestry: &mut Vec<&'a XastElement>,
    results: &mut Vec<&'a XastChild>,
) {
    for child in children {
        if let XastChild::Element(element) = child {
            if selectors
                .iter()
                .any(|selector| selector_matches(selector, element, ancestry.as_slice()))
            {
                results.push(child);
            }
            ancestry.push(element);
            collect_matches(&element.children, selectors, ancestry, results);
            ancestry.pop();
        }
    }
}

fn selector_matches(
    selector: &ParsedSelector,
    node: &XastElement,
    ancestry: &[&XastElement],
) -> bool {
    if selector.parts.is_empty() {
        return false;
    }
    let last_index = selector.parts.len() - 1;
    if !matches_compound(node, &selector.parts[last_index].compound) {
        return false;
    }
    if last_index == 0 {
        return true;
    }
    match_selector_ancestors(selector, last_index - 1, ancestry)
}

fn match_selector_ancestors(
    selector: &ParsedSelector,
    part_index: usize,
    ancestry: &[&XastElement],
) -> bool {
    let part = &selector.parts[part_index];
    let combinator = selector.parts[part_index + 1]
        .combinator
        .unwrap_or(Combinator::Descendant);

    match combinator {
        Combinator::Child => ancestry.last().is_some_and(|parent| {
            matches_compound(parent, &part.compound)
                && (part_index == 0
                    || match_selector_ancestors(
                        selector,
                        part_index - 1,
                        &ancestry[..ancestry.len() - 1],
                    ))
        }),
        Combinator::Descendant => {
            let mut index = ancestry.len();
            while index > 0 {
                index -= 1;
                if matches_compound(ancestry[index], &part.compound)
                    && (part_index == 0
                        || match_selector_ancestors(selector, part_index - 1, &ancestry[..index]))
                {
                    return true;
                }
            }
            false
        }
    }
}

fn matches_compound(node: &XastElement, compound: &CompoundSelector) -> bool {
    if !compound.universal
        && let Some(tag) = &compound.tag
        && node.name != *tag
    {
        return false;
    }

    if let Some(id) = &compound.id
        && node.get_attribute("id") != Some(id.as_str())
    {
        return false;
    }

    for class_name in &compound.classes {
        let Some(classes) = node.get_attribute("class") else {
            return false;
        };
        if !classes
            .split_ascii_whitespace()
            .any(|class| class == class_name)
        {
            return false;
        }
    }

    for attribute in &compound.attributes {
        let Some(value) = node.get_attribute(attribute.name.as_str()) else {
            return false;
        };
        if let Some(expected) = &attribute.value
            && value != expected
        {
            return false;
        }
    }

    true
}

fn parse_selectors(selector: &str) -> Vec<ParsedSelector> {
    split_top_level(selector, ',')
        .into_iter()
        .filter_map(|group| parse_selector_group(group.trim()))
        .collect()
}

fn parse_selector_group(group: &str) -> Option<ParsedSelector> {
    let mut parts = Vec::<SelectorPart>::new();
    let mut buffer = String::new();
    let mut bracket_depth = 0usize;
    let mut quote = None::<char>;
    let mut pending_combinator = None::<Combinator>;
    let mut chars = group.chars().peekable();

    while let Some(ch) = chars.next() {
        if let Some(current_quote) = quote {
            buffer.push(ch);
            if ch == current_quote {
                quote = None;
            }
            continue;
        }

        match ch {
            '"' | '\'' => {
                quote = Some(ch);
                buffer.push(ch);
            }
            '[' => {
                bracket_depth += 1;
                buffer.push(ch);
            }
            ']' => {
                bracket_depth = bracket_depth.saturating_sub(1);
                buffer.push(ch);
            }
            '>' if bracket_depth == 0 => {
                if !buffer.trim().is_empty() {
                    push_selector_part(&mut parts, &mut buffer, pending_combinator.take());
                }
                pending_combinator = Some(Combinator::Child);
            }
            c if c.is_ascii_whitespace() && bracket_depth == 0 => {
                if !buffer.trim().is_empty() {
                    push_selector_part(&mut parts, &mut buffer, pending_combinator.take());
                }
                while chars.peek().is_some_and(char::is_ascii_whitespace) {
                    chars.next();
                }
                if pending_combinator.is_none() && chars.peek().is_some_and(|next| *next != '>') {
                    pending_combinator = Some(Combinator::Descendant);
                }
            }
            _ => buffer.push(ch),
        }
    }

    push_selector_part(&mut parts, &mut buffer, pending_combinator.take());
    if parts.is_empty() {
        None
    } else {
        Some(ParsedSelector { parts })
    }
}

fn push_selector_part(
    parts: &mut Vec<SelectorPart>,
    buffer: &mut String,
    combinator: Option<Combinator>,
) {
    let trimmed = buffer.trim();
    if trimmed.is_empty() {
        buffer.clear();
        return;
    }
    parts.push(SelectorPart {
        combinator,
        compound: parse_compound(trimmed),
    });
    buffer.clear();
}

fn parse_compound(input: &str) -> CompoundSelector {
    let mut compound = CompoundSelector::default();
    let chars = input.chars().collect::<Vec<_>>();
    let mut index = 0usize;

    if chars.first() == Some(&'*') {
        compound.universal = true;
        index = 1;
    } else if chars
        .first()
        .is_some_and(|char| char.is_ascii_alphabetic() || *char == '_')
    {
        let start = index;
        while index < chars.len()
            && (chars[index].is_ascii_alphanumeric() || matches!(chars[index], '-' | '_' | ':'))
        {
            index += 1;
        }
        compound.tag = Some(chars[start..index].iter().collect());
    }

    while index < chars.len() {
        match chars[index] {
            '#' => {
                index += 1;
                let start = index;
                while index < chars.len()
                    && (chars[index].is_ascii_alphanumeric()
                        || matches!(chars[index], '-' | '_' | ':'))
                {
                    index += 1;
                }
                compound.id = Some(chars[start..index].iter().collect());
            }
            '.' => {
                index += 1;
                let start = index;
                while index < chars.len()
                    && (chars[index].is_ascii_alphanumeric() || matches!(chars[index], '-' | '_'))
                {
                    index += 1;
                }
                compound.classes.push(chars[start..index].iter().collect());
            }
            '[' => {
                index += 1;
                let start = index;
                while index < chars.len() && chars[index] != ']' {
                    index += 1;
                }
                let raw = chars[start..index].iter().collect::<String>();
                if index < chars.len() {
                    index += 1;
                }
                compound
                    .attributes
                    .push(parse_attribute_selector(raw.trim()));
            }
            _ => index += 1,
        }
    }

    compound
}

fn parse_attribute_selector(input: &str) -> AttributeSelector {
    if let Some((name, value)) = input.split_once('=') {
        AttributeSelector {
            name: name.trim().to_string(),
            value: Some(
                value
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string(),
            ),
        }
    } else {
        AttributeSelector {
            name: input.trim().to_string(),
            value: None,
        }
    }
}

fn split_top_level(input: &str, separator: char) -> Vec<String> {
    let mut parts = Vec::<String>::new();
    let mut current = String::new();
    let mut bracket_depth = 0usize;
    let mut quote = None::<char>;

    for ch in input.chars() {
        if let Some(current_quote) = quote {
            current.push(ch);
            if ch == current_quote {
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
            c if c == separator && bracket_depth == 0 => {
                parts.push(std::mem::take(&mut current));
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        parts.push(current);
    }
    parts
}
