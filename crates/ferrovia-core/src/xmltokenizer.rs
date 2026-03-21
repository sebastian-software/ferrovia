#![allow(dead_code)]

use xmlparser::{ElementEnd, Token, Tokenizer};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum XmlTokenSummary {
    Declaration {
        version: String,
        encoding: Option<String>,
        standalone: Option<bool>,
    },
    ProcessingInstruction {
        target: String,
        content: Option<String>,
    },
    Comment(String),
    DoctypeStart {
        name: String,
    },
    EmptyDoctype {
        name: String,
    },
    EntityDeclaration {
        name: String,
    },
    DoctypeEnd,
    ElementStart {
        name: String,
    },
    Attribute {
        name: String,
        value: String,
    },
    ElementOpen,
    ElementClose {
        name: String,
    },
    ElementEmpty,
    Text(String),
    Cdata(String),
}

pub(crate) fn summarize(input: &str) -> std::result::Result<Vec<XmlTokenSummary>, String> {
    Tokenizer::from(input)
        .map(|token| token.map(summarize_token).map_err(|error| error.to_string()))
        .collect()
}

fn summarize_token(token: Token<'_>) -> XmlTokenSummary {
    match token {
        Token::Declaration {
            version,
            encoding,
            standalone,
            ..
        } => XmlTokenSummary::Declaration {
            version: version.as_str().to_string(),
            encoding: encoding.map(|value| value.as_str().to_string()),
            standalone,
        },
        Token::ProcessingInstruction {
            target, content, ..
        } => XmlTokenSummary::ProcessingInstruction {
            target: target.as_str().to_string(),
            content: content.map(|value| value.as_str().to_string()),
        },
        Token::Comment { text, .. } => XmlTokenSummary::Comment(text.as_str().to_string()),
        Token::DtdStart { name, .. } => XmlTokenSummary::DoctypeStart {
            name: name.as_str().to_string(),
        },
        Token::EmptyDtd { name, .. } => XmlTokenSummary::EmptyDoctype {
            name: name.as_str().to_string(),
        },
        Token::DtdEnd { .. } => XmlTokenSummary::DoctypeEnd,
        Token::EntityDeclaration { name, .. } => XmlTokenSummary::EntityDeclaration {
            name: name.as_str().to_string(),
        },
        Token::ElementStart { prefix, local, .. } => XmlTokenSummary::ElementStart {
            name: qualified_name(prefix.as_str(), local.as_str()),
        },
        Token::Attribute {
            prefix,
            local,
            value,
            ..
        } => XmlTokenSummary::Attribute {
            name: qualified_name(prefix.as_str(), local.as_str()),
            value: value.as_str().to_string(),
        },
        Token::ElementEnd { end, .. } => match end {
            ElementEnd::Open => XmlTokenSummary::ElementOpen,
            ElementEnd::Close(prefix, local) => XmlTokenSummary::ElementClose {
                name: qualified_name(prefix.as_str(), local.as_str()),
            },
            ElementEnd::Empty => XmlTokenSummary::ElementEmpty,
        },
        Token::Text { text } => XmlTokenSummary::Text(text.as_str().to_string()),
        Token::Cdata { text, .. } => XmlTokenSummary::Cdata(text.as_str().to_string()),
    }
}

fn qualified_name(prefix: &str, local: &str) -> String {
    if prefix.is_empty() {
        local.to_string()
    } else {
        format!("{prefix}:{local}")
    }
}

#[cfg(test)]
mod tests {
    use super::{XmlTokenSummary, summarize};

    #[test]
    fn xmlparser_spike_covers_svg_structural_tokens() {
        let tokens = summarize(
            r#"<?xml version="1.0" encoding="UTF-8"?><!--note--><?proc keep?><!DOCTYPE svg [<!ENTITY test "x">]><svg xmlns="http://www.w3.org/2000/svg"><![CDATA[a<b]]><g id="hero">text</g><path/></svg>"#,
        )
        .expect("tokenize");

        assert!(matches!(
            tokens[0],
            XmlTokenSummary::Declaration {
                ref version,
                ref encoding,
                standalone: None
            } if version == "1.0" && encoding.as_deref() == Some("UTF-8")
        ));
        assert!(tokens.iter().any(|token| matches!(token, XmlTokenSummary::Comment(text) if text == "note")));
        assert!(tokens.iter().any(|token| matches!(token, XmlTokenSummary::ProcessingInstruction { target, content } if target == "proc" && content.as_deref() == Some("keep"))));
        assert!(tokens.iter().any(|token| matches!(token, XmlTokenSummary::DoctypeStart { name } if name == "svg")));
        assert!(tokens.iter().any(|token| matches!(token, XmlTokenSummary::Cdata(text) if text == "a<b")));
        assert!(tokens.iter().any(|token| matches!(token, XmlTokenSummary::ElementStart { name } if name == "svg")));
        assert!(tokens.iter().any(|token| matches!(token, XmlTokenSummary::Attribute { name, value } if name == "id" && value == "hero")));
        assert!(tokens.iter().any(|token| matches!(token, XmlTokenSummary::ElementEmpty)));
    }

    #[test]
    fn xmlparser_spike_rejects_malformed_comment() {
        let error = summarize("<!-- broken -- comment -->").expect_err("invalid xml");
        assert!(!error.is_empty());
    }
}
