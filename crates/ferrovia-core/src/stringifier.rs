use crate::plugins::_collections::is_text_elem;
use crate::types::{
    StringifyOptions, XastAttribute, XastCdata, XastChild, XastComment, XastDoctype, XastElement,
    XastInstruction, XastRoot, XastText,
};

#[must_use]
pub fn stringify_svg(data: &XastRoot, user_options: Option<StringifyOptions>) -> String {
    let config = user_options.unwrap_or_default();
    let state = State {
        indent: " ".repeat(config.indent),
        text_context: None,
        indent_level: 0,
        pretty: config.pretty,
    };
    stringify_node(data, &state)
}

#[derive(Debug, Clone)]
struct State {
    indent: String,
    text_context: Option<String>,
    indent_level: usize,
    pretty: bool,
}

fn stringify_node(data: &XastRoot, state: &State) -> String {
    let mut svg = String::new();
    let mut next_state = state.clone();
    next_state.indent_level += 1;
    for item in &data.children {
        svg.push_str(stringify_child(item, &next_state).as_str());
    }
    svg
}

fn stringify_child(child: &XastChild, state: &State) -> String {
    match child {
        XastChild::Element(node) => stringify_element(node, state),
        XastChild::Text(node) => stringify_text(node, state),
        XastChild::Doctype(node) => stringify_doctype(node),
        XastChild::Instruction(node) => stringify_instruction(node),
        XastChild::Comment(node) => stringify_comment(node),
        XastChild::Cdata(node) => stringify_cdata(node, state),
    }
}

fn stringify_doctype(node: &XastDoctype) -> String {
    format!("<!DOCTYPE{}>", node.doctype)
}

fn stringify_instruction(node: &XastInstruction) -> String {
    format!("<?{} {}?>", node.name, node.value)
}

fn stringify_comment(node: &XastComment) -> String {
    format!("<!--{}-->", node.value)
}

fn stringify_cdata(node: &XastCdata, state: &State) -> String {
    format!("{}<![CDATA[{}]]>", create_indent(state), node.value)
}

fn stringify_element(node: &XastElement, state: &State) -> String {
    if node.children.is_empty() {
        return format!(
            "{}<{}{}{}/>",
            create_indent(state),
            node.name,
            stringify_attributes(&node.attributes),
            ""
        );
    }

    let mut next_state = state.clone();
    if is_text_elem(node.name.as_str()) {
        next_state.text_context = Some(node.name.clone());
    }
    next_state.indent_level += 1;

    let mut content = String::new();
    for child in &node.children {
        content.push_str(stringify_child(child, &next_state).as_str());
    }
    format!(
        "{}<{}{}>{}</{}>",
        create_indent(state),
        node.name,
        stringify_attributes(&node.attributes),
        content,
        node.name
    )
}

fn stringify_text(node: &XastText, state: &State) -> String {
    let prefix = create_indent(state);
    format!("{prefix}{}", encode_text(node.value.as_str()))
}

fn stringify_attributes(attributes: &[XastAttribute]) -> String {
    let mut serialized = String::new();
    for attribute in attributes {
        serialized.push(' ');
        serialized.push_str(attribute.name.as_str());
        serialized.push_str("=\"");
        serialized.push_str(encode_attribute_value(attribute.value.as_str()).as_str());
        serialized.push('"');
    }
    serialized
}

fn create_indent(state: &State) -> String {
    if state.pretty && state.text_context.is_none() {
        state.indent.repeat(state.indent_level.saturating_sub(1))
    } else {
        String::new()
    }
}

fn encode_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('\'', "&apos;")
        .replace('"', "&quot;")
        .replace('>', "&gt;")
        .replace('<', "&lt;")
}

fn encode_attribute_value(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('>', "&gt;")
        .replace('<', "&lt;")
}
