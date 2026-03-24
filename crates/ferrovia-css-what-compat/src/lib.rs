#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Combinator {
    Descendant,
    Child,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeSelector {
    pub name: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CompoundSelector {
    pub tag: Option<String>,
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub attributes: Vec<AttributeSelector>,
    pub universal: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectorToken {
    pub combinator: Option<Combinator>,
    pub compound: CompoundSelector,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectorGroup {
    pub tokens: Vec<SelectorToken>,
}

#[must_use]
pub fn parse(input: &str) -> Vec<SelectorGroup> {
    split_top_level(input, ',')
        .into_iter()
        .filter_map(|group| parse_selector_group(group.trim()))
        .collect()
}

fn parse_selector_group(group: &str) -> Option<SelectorGroup> {
    let mut tokens = Vec::<SelectorToken>::new();
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
                    push_selector_token(&mut tokens, &mut buffer, pending_combinator.take());
                }
                pending_combinator = Some(Combinator::Child);
            }
            c if c.is_ascii_whitespace() && bracket_depth == 0 => {
                if !buffer.trim().is_empty() {
                    push_selector_token(&mut tokens, &mut buffer, pending_combinator.take());
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

    push_selector_token(&mut tokens, &mut buffer, pending_combinator.take());
    if tokens.is_empty() {
        None
    } else {
        Some(SelectorGroup { tokens })
    }
}

fn push_selector_token(
    tokens: &mut Vec<SelectorToken>,
    buffer: &mut String,
    combinator: Option<Combinator>,
) {
    let trimmed = buffer.trim();
    if trimmed.is_empty() {
        buffer.clear();
        return;
    }
    tokens.push(SelectorToken {
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
