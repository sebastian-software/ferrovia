use ferrovia_sax_compat::{SaxEvent, parse as sax_parse};

use crate::error::{FerroviaError, Result};
use crate::plugins::_collections::is_text_elem;
use crate::types::{
    XastAttribute, XastCdata, XastChild, XastComment, XastDoctype, XastElement, XastInstruction,
    XastRoot, XastText,
};

/// Parse an SVG string into the rewrite's xast-like tree.
///
/// # Errors
///
/// Returns an error when the SAX compatibility layer cannot tokenize the input
/// or when the input has unbalanced closing tags.
pub fn parse_svg(data: &str, from: Option<&str>) -> Result<XastRoot> {
    let events = sax_parse(data, from).map_err(|error| FerroviaError::Parse {
        position: error.position,
        message: error.message,
    })?;
    let mut root = XastRoot::new();
    let mut stack = Vec::<XastElement>::new();

    for event in events {
        match event {
            SaxEvent::Doctype { value } => push_child(
                &mut root,
                &mut stack,
                XastChild::Doctype(XastDoctype {
                    name: "svg".to_string(),
                    doctype: value,
                }),
            ),
            SaxEvent::Instruction { name, value } => push_child(
                &mut root,
                &mut stack,
                XastChild::Instruction(XastInstruction { name, value }),
            ),
            SaxEvent::Comment { value } => push_child(
                &mut root,
                &mut stack,
                XastChild::Comment(XastComment {
                    value: value.trim().to_string(),
                }),
            ),
            SaxEvent::Cdata { value } => push_child(
                &mut root,
                &mut stack,
                XastChild::Cdata(XastCdata { value }),
            ),
            SaxEvent::OpenTag {
                name,
                attributes,
                self_closing,
            } => {
                let element = XastElement {
                    name,
                    attributes: attributes
                        .into_iter()
                        .map(|(name, value)| XastAttribute { name, value })
                        .collect(),
                    children: Vec::new(),
                };
                if self_closing {
                    push_child(&mut root, &mut stack, XastChild::Element(element));
                } else {
                    stack.push(element);
                }
            }
            SaxEvent::CloseTag => {
                let element = stack.pop().ok_or_else(|| FerroviaError::Parse {
                    position: 0,
                    message: "unexpected close tag".to_string(),
                })?;
                push_child(&mut root, &mut stack, XastChild::Element(element));
            }
            SaxEvent::Text { value } => {
                let in_text_context = stack
                    .last()
                    .is_some_and(|element| is_text_elem(element.name.as_str()));
                if in_text_context {
                    push_child(&mut root, &mut stack, XastChild::Text(XastText { value }));
                } else {
                    let trimmed = value.trim();
                    if !trimmed.is_empty() {
                        push_child(
                            &mut root,
                            &mut stack,
                            XastChild::Text(XastText {
                                value: trimmed.to_string(),
                            }),
                        );
                    }
                }
            }
        }
    }

    if !stack.is_empty() {
        return Err(FerroviaError::Parse {
            position: data.len(),
            message: "unexpected end of input".to_string(),
        });
    }

    Ok(root)
}

fn push_child(root: &mut XastRoot, stack: &mut [XastElement], child: XastChild) {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(child);
    } else {
        root.children.push(child);
    }
}
