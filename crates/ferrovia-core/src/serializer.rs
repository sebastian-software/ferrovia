use crate::ast::{Attribute, Document, NodeId, NodeKind, QuoteStyle};
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
                out.push_str(trimmed);
                newline(out, options);
            } else {
                out.push_str(text);
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

            let has_children = doc.node(id).first_child.is_some();
            if element.self_closing && !has_children {
                out.push_str("/>");
                newline(out, options);
                return;
            }

            out.push('>');
            if options.pretty && has_children {
                newline(out, options);
            }
            for child in doc.children(id) {
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
    match attribute.quote {
        QuoteStyle::Double => {
            out.push('"');
            out.push_str(&attribute.value);
            out.push('"');
        }
        QuoteStyle::Single => {
            out.push('\'');
            out.push_str(&attribute.value);
            out.push('\'');
        }
    }
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
