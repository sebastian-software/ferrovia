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
            if options.pretty {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    return;
                }
                indent(out, options, depth);
                out.push_str(&escape_text(trimmed));
                newline(out, options);
            } else {
                out.push_str(&escape_text(text));
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

            let children: Vec<_> = doc.children(id).collect();
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
