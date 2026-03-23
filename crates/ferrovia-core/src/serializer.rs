use std::borrow::Cow;

use crate::ast::{Attribute, Document, NodeId, NodeKind};
use crate::config::Js2Svg;

#[must_use]
pub fn serialize(doc: &Document, options: &Js2Svg) -> String {
    let mut out = String::new();
    for child in doc.children(doc.root_id()) {
        serialize_node(doc, child, &mut out, options, 0);
    }
    out
}

fn serialize_node(doc: &Document, id: NodeId, out: &mut String, options: &Js2Svg, depth: usize) {
    match &doc.node(id).kind {
        NodeKind::Document => {}
        NodeKind::XmlDecl(decl) => {
            indent(out, options, depth);
            out.push_str("<?xml");
            for attribute in &decl.attributes {
                serialize_attribute(attribute, out);
            }
            out.push_str("?>");
            newline(out, options);
        }
        NodeKind::Doctype(data) => {
            indent(out, options, depth);
            out.push_str("<!DOCTYPE ");
            out.push_str(data);
            out.push('>');
            newline(out, options);
        }
        NodeKind::Comment(data) => {
            indent(out, options, depth);
            out.push_str("<!--");
            out.push_str(data);
            out.push_str("-->");
            newline(out, options);
        }
        NodeKind::Text(text) => {
            let Some(text) = normalized_text_for_context(doc, id, text) else {
                return;
            };
            if options.pretty {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    return;
                }
                indent(out, options, depth);
                out.push_str(&escape_text(trimmed));
                newline(out, options);
            } else {
                out.push_str(&escape_text(text.as_ref()));
            }
        }
        NodeKind::Cdata(data) => {
            indent(out, options, depth);
            out.push_str("<![CDATA[");
            out.push_str(data);
            out.push_str("]]>");
            newline(out, options);
        }
        NodeKind::Element(element) => {
            indent(out, options, depth);
            out.push('<');
            out.push_str(&element.name);
            for attribute in &element.attributes {
                serialize_attribute(attribute, out);
            }

            let children: Vec<_> = doc
                .children(id)
                .filter(|child_id| should_serialize_node(doc, *child_id))
                .collect();
            let has_children = !children.is_empty();
            if !has_children {
                out.push_str("/>");
                newline(out, options);
                return;
            }

            if options.pretty
                && children.len() == 1
                && let NodeKind::Text(text) = &doc.node(children[0]).kind
                && !text.is_empty()
                && text.trim() == text
            {
                out.push('>');
                out.push_str(text);
                out.push_str("</");
                out.push_str(&element.name);
                out.push('>');
                newline(out, options);
                return;
            }

            out.push('>');
            if options.pretty && has_children {
                newline(out, options);
            }
            for child in children {
                serialize_node(doc, child, out, options, depth + 1);
            }
            if options.pretty && has_children {
                indent(out, options, depth);
            }
            out.push_str("</");
            out.push_str(&element.name);
            out.push('>');
            newline(out, options);
        }
    }
}

fn should_serialize_node(doc: &Document, id: NodeId) -> bool {
    match &doc.node(id).kind {
        NodeKind::Text(text) => normalized_text_for_context(doc, id, text).is_some(),
        _ => true,
    }
}

fn normalized_text_for_context<'a>(doc: &'a Document, id: NodeId, text: &'a str) -> Option<Cow<'a, str>> {
    if text.trim().is_empty() && !should_preserve_whitespace_text(doc, id) {
        return None;
    }

    if should_trim_script_outer_whitespace(doc, id, text) {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return None;
        }
        return Some(Cow::Owned(trimmed.to_string()));
    }

    Some(Cow::Borrowed(text))
}

fn should_preserve_whitespace_text(doc: &Document, id: NodeId) -> bool {
    let Some(parent_id) = doc.node(id).parent else {
        return false;
    };
    if parent_id == doc.root_id() {
        return false;
    }

    matches!(
        &doc.node(parent_id).kind,
        NodeKind::Element(element) if element.name == "a"
    )
}

fn should_trim_script_outer_whitespace(doc: &Document, id: NodeId, text: &str) -> bool {
    if !text.contains(['\n', '\r', '\t']) || text.trim().is_empty() {
        return false;
    }

    let Some(parent_id) = doc.node(id).parent else {
        return false;
    };

    let NodeKind::Element(parent) = &doc.node(parent_id).kind else {
        return false;
    };

    parent.name == "script"
}

fn serialize_attribute(attribute: &Attribute, out: &mut String) {
    out.push(' ');
    out.push_str(&attribute.name);
    out.push('=');
    out.push('"');
    out.push_str(&attribute.value.replace('"', "&quot;"));
    out.push('"');
}

fn escape_text(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'&'
            && let Some(end) = value[index + 1..].find(';')
        {
            let candidate = &value[index + 1..index + 1 + end];
            if is_entity_reference(candidate) {
                escaped.push_str(&value[index..=index + end + 1]);
                index += end + 2;
                continue;
            }
        }

        let ch = value[index..].chars().next().expect("char boundary");
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            _ => escaped.push(ch),
        }
        index += ch.len_utf8();
    }
    escaped
}

fn is_entity_reference(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    if let Some(rest) = value.strip_prefix("#x").or_else(|| value.strip_prefix("#X")) {
        return !rest.is_empty() && rest.chars().all(|ch| ch.is_ascii_hexdigit());
    }
    if let Some(rest) = value.strip_prefix('#') {
        return !rest.is_empty() && rest.chars().all(|ch| ch.is_ascii_digit());
    }
    value.chars().all(|ch| ch.is_ascii_alphanumeric())
}

fn indent(out: &mut String, options: &Js2Svg, depth: usize) {
    if options.pretty {
        for _ in 0..(depth * options.indent) {
            out.push(' ');
        }
    }
}

fn newline(out: &mut String, options: &Js2Svg) {
    if options.pretty {
        out.push('\n');
    }
}
