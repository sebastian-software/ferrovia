use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaxEvent {
    Doctype {
        value: String,
    },
    Instruction {
        name: String,
        value: String,
    },
    Comment {
        value: String,
    },
    Cdata {
        value: String,
    },
    OpenTag {
        name: String,
        attributes: Vec<(String, String)>,
        self_closing: bool,
    },
    CloseTag,
    Text {
        value: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{message}")]
pub struct SaxCompatError {
    pub position: usize,
    pub message: String,
}

/// Parse an XML/SVG string into a stream of SAX-like events for the direct-port
/// rewrite.
///
/// # Errors
///
/// Returns an error when the input contains an unterminated or malformed XML
/// construct that this compat layer currently understands.
pub fn parse(data: &str, _from: Option<&str>) -> Result<Vec<SaxEvent>, SaxCompatError> {
    let mut events = Vec::new();
    let bytes = data.as_bytes();
    let mut index = 0usize;

    while index < bytes.len() {
        if bytes[index] != b'<' {
            let start = index;
            while index < bytes.len() && bytes[index] != b'<' {
                index += 1;
            }
            events.push(SaxEvent::Text {
                value: data[start..index].to_string(),
            });
            continue;
        }

        if data[index..].starts_with("<!--") {
            let end = data[index + 4..]
                .find("-->")
                .ok_or_else(|| error(index, "unterminated comment"))?
                + index
                + 4;
            events.push(SaxEvent::Comment {
                value: data[index + 4..end].to_string(),
            });
            index = end + 3;
            continue;
        }

        if data[index..].starts_with("<![CDATA[") {
            let end = data[index + 9..]
                .find("]]>")
                .ok_or_else(|| error(index, "unterminated cdata"))?
                + index
                + 9;
            events.push(SaxEvent::Cdata {
                value: data[index + 9..end].to_string(),
            });
            index = end + 3;
            continue;
        }

        if data[index..].starts_with("<!DOCTYPE") {
            let end = data[index..]
                .find('>')
                .ok_or_else(|| error(index, "unterminated doctype"))?
                + index;
            events.push(SaxEvent::Doctype {
                value: data[index + "<!DOCTYPE".len()..end].to_string(),
            });
            index = end + 1;
            continue;
        }

        if data[index..].starts_with("<?") {
            let end = data[index + 2..]
                .find("?>")
                .ok_or_else(|| error(index, "unterminated instruction"))?
                + index
                + 2;
            let body = data[index + 2..end].trim();
            let (name, value) = body
                .split_once(char::is_whitespace)
                .map_or((body, ""), |(name, value)| (name, value.trim()));
            events.push(SaxEvent::Instruction {
                name: name.to_string(),
                value: value.to_string(),
            });
            index = end + 2;
            continue;
        }

        if data[index..].starts_with("</") {
            let end = data[index..]
                .find('>')
                .ok_or_else(|| error(index, "unterminated close tag"))?
                + index;
            events.push(SaxEvent::CloseTag);
            index = end + 1;
            continue;
        }

        let end = data[index..]
            .find('>')
            .ok_or_else(|| error(index, "unterminated open tag"))?
            + index;
        let open_tag = &data[index + 1..end];
        let self_closing = open_tag.trim_end().ends_with('/');
        let tag_content = if self_closing {
            open_tag.trim_end_matches('/').trim_end()
        } else {
            open_tag
        };
        let (name, attributes) = parse_tag(tag_content, index)?;
        events.push(SaxEvent::OpenTag {
            name,
            attributes,
            self_closing,
        });
        index = end + 1;
    }

    Ok(events)
}

fn parse_tag(
    content: &str,
    position: usize,
) -> Result<(String, Vec<(String, String)>), SaxCompatError> {
    let mut cursor = 0usize;
    let bytes = content.as_bytes();
    while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
        cursor += 1;
    }
    let name_start = cursor;
    while cursor < bytes.len()
        && !bytes[cursor].is_ascii_whitespace()
        && bytes[cursor] != b'/'
        && bytes[cursor] != b'>'
    {
        cursor += 1;
    }
    let name = content[name_start..cursor].to_string();
    let mut attributes = Vec::new();

    loop {
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() {
            break;
        }
        let attr_start = cursor;
        while cursor < bytes.len() && !bytes[cursor].is_ascii_whitespace() && bytes[cursor] != b'='
        {
            cursor += 1;
        }
        let attr_name = content[attr_start..cursor].to_string();
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if bytes.get(cursor) != Some(&b'=') {
            return Err(error(position + cursor, "expected = in attribute"));
        }
        cursor += 1;
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        let quote = *bytes
            .get(cursor)
            .ok_or_else(|| error(position + cursor, "expected quoted attribute value"))?;
        if quote != b'"' && quote != b'\'' {
            return Err(error(position + cursor, "expected quoted attribute value"));
        }
        cursor += 1;
        let value_start = cursor;
        while cursor < bytes.len() && bytes[cursor] != quote {
            cursor += 1;
        }
        if cursor >= bytes.len() {
            return Err(error(position + cursor, "unterminated attribute value"));
        }
        let attr_value = content[value_start..cursor].to_string();
        cursor += 1;
        attributes.push((attr_name, attr_value));
    }

    Ok((name, attributes))
}

fn error(position: usize, message: &str) -> SaxCompatError {
    SaxCompatError {
        position,
        message: message.to_string(),
    }
}
