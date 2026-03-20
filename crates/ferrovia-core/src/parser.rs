use memchr::memchr;

use crate::ast::{Attribute, Document, Element, NodeId, NodeKind, QuoteStyle, XmlDecl};
use crate::error::{FerroviaError, Result};

/// Parse an SVG/XML string into the arena-backed document model.
///
/// # Errors
///
/// Returns an error if the input contains malformed tags, unterminated literals,
/// or mismatched closing tags.
pub fn parse(svg: &str) -> Result<Document> {
    let parser = Parser::new(svg);
    parser.parse()
}

struct Parser<'a> {
    input: &'a str,
    bytes: &'a [u8],
    position: usize,
    doc: Document,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            bytes: input.as_bytes(),
            position: 0,
            doc: Document::new(),
        }
    }

    fn parse(mut self) -> Result<Document> {
        self.parse_children(self.doc.root_id(), None)?;
        Ok(self.doc)
    }

    fn parse_children(&mut self, parent: NodeId, closing_tag: Option<&str>) -> Result<()> {
        while self.position < self.bytes.len() {
            if self.starts_with("</") {
                let name = self.parse_end_tag()?;
                if let Some(expected) = closing_tag {
                    if name == expected {
                        return Ok(());
                    }
                    return Err(self.error(format!(
                        "expected closing tag </{expected}> but found </{name}>"
                    )));
                }
                return Err(self.error(format!("unexpected closing tag </{name}>")));
            }

            if self.starts_with("<?xml") {
                let decl = self.parse_xml_decl()?;
                self.doc.append_child(parent, NodeKind::XmlDecl(decl));
                continue;
            }

            if self.starts_with("<?") {
                let data = self.parse_processing_instruction()?;
                self.doc
                    .append_child(parent, NodeKind::XmlDecl(XmlDecl { attributes: data }));
                continue;
            }

            if self.starts_with("<!--") {
                let comment = self.parse_comment()?;
                self.doc.append_child(parent, NodeKind::Comment(comment));
                continue;
            }

            if self.starts_with("<![CDATA[") {
                let cdata = self.parse_cdata()?;
                self.doc.append_child(parent, NodeKind::Cdata(cdata));
                continue;
            }

            if self.starts_with("<!DOCTYPE") {
                let doctype = self.parse_doctype()?;
                self.doc.append_child(parent, NodeKind::Doctype(doctype));
                continue;
            }

            if self.current_byte() == Some(b'<') {
                let (element, self_closing, name) = self.parse_start_tag()?;
                let id = self.doc.append_child(parent, NodeKind::Element(element));
                if !self_closing {
                    self.parse_children(id, Some(name.as_str()))?;
                }
                continue;
            }

            let text = self.parse_text();
            if !text.trim().is_empty() {
                self.doc.append_child(parent, NodeKind::Text(text));
            }
        }

        if let Some(expected) = closing_tag {
            return Err(self.error(format!("missing closing tag </{expected}>")));
        }

        Ok(())
    }

    fn parse_xml_decl(&mut self) -> Result<XmlDecl> {
        self.consume("<?xml")?;
        let attributes = self.parse_attributes_until("?>")?;
        self.consume("?>")?;
        Ok(XmlDecl { attributes })
    }

    fn parse_processing_instruction(&mut self) -> Result<Vec<Attribute>> {
        self.consume("<?")?;
        let attributes = self.parse_attributes_until("?>")?;
        self.consume("?>")?;
        Ok(attributes)
    }

    fn parse_comment(&mut self) -> Result<String> {
        self.consume("<!--")?;
        self.consume_until("-->")
    }

    fn parse_cdata(&mut self) -> Result<String> {
        self.consume("<![CDATA[")?;
        self.consume_until("]]>")
    }

    fn parse_doctype(&mut self) -> Result<String> {
        self.consume("<!DOCTYPE")?;
        let start = self.position;
        let end = self
            .find_byte(b'>')
            .ok_or_else(|| self.error("unterminated doctype"))?;
        self.position = end + 1;
        Ok(self.input[start..end].trim().to_string())
    }

    fn parse_start_tag(&mut self) -> Result<(Element, bool, String)> {
        self.consume("<")?;
        let name = self.parse_name()?;
        let attributes = self.parse_attributes_until_tag_end()?;
        let self_closing = if self.starts_with("/>") {
            self.position += 2;
            true
        } else {
            self.consume(">")?;
            false
        };

        let element = Element {
            name: name.clone(),
            attributes,
            self_closing,
        };
        Ok((element, self_closing, name))
    }

    fn parse_end_tag(&mut self) -> Result<String> {
        self.consume("</")?;
        let name = self.parse_name()?;
        self.consume_optional_whitespace();
        self.consume(">")?;
        Ok(name)
    }

    fn parse_attributes_until_tag_end(&mut self) -> Result<Vec<Attribute>> {
        let mut attributes = Vec::new();
        loop {
            self.consume_optional_whitespace();
            if self.starts_with("/>") || self.starts_with(">") {
                break;
            }
            attributes.push(self.parse_attribute()?);
        }
        Ok(attributes)
    }

    fn parse_attributes_until(&mut self, terminator: &str) -> Result<Vec<Attribute>> {
        let mut attributes = Vec::new();
        loop {
            self.consume_optional_whitespace();
            if self.starts_with(terminator) {
                break;
            }
            attributes.push(self.parse_attribute()?);
        }
        Ok(attributes)
    }

    fn parse_attribute(&mut self) -> Result<Attribute> {
        let name = self.parse_name()?;
        self.consume_optional_whitespace();
        self.consume("=")?;
        self.consume_optional_whitespace();
        let quote = match self.current_byte() {
            Some(b'"') => QuoteStyle::Double,
            Some(b'\'') => QuoteStyle::Single,
            _ => return Err(self.error("expected quoted attribute value")),
        };
        self.position += 1;
        let start = self.position;
        while let Some(byte) = self.current_byte() {
            if byte == Self::quote_byte(quote) {
                let value = self.input[start..self.position].to_string();
                self.position += 1;
                return Ok(Attribute { name, value, quote });
            }
            self.position += 1;
        }
        Err(self.error("unterminated attribute value"))
    }

    fn parse_name(&mut self) -> Result<String> {
        let start = self.position;
        while let Some(byte) = self.current_byte() {
            if matches!(
                byte,
                b' ' | b'\t' | b'\r' | b'\n' | b'/' | b'>' | b'=' | b'?'
            ) {
                break;
            }
            self.position += 1;
        }
        if self.position == start {
            return Err(self.error("expected name"));
        }
        Ok(self.input[start..self.position].to_string())
    }

    fn parse_text(&mut self) -> String {
        let start = self.position;
        if let Some(offset) = memchr(b'<', &self.bytes[self.position..]) {
            self.position += offset;
        } else {
            self.position = self.bytes.len();
        }
        self.input[start..self.position].to_string()
    }

    fn consume_until(&mut self, needle: &str) -> Result<String> {
        let start = self.position;
        let haystack = &self.input[self.position..];
        let Some(relative) = haystack.find(needle) else {
            return Err(self.error(format!("unterminated sequence {needle}")));
        };
        let end = self.position + relative;
        self.position = end + needle.len();
        Ok(self.input[start..end].to_string())
    }

    fn consume_optional_whitespace(&mut self) {
        while matches!(self.current_byte(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            self.position += 1;
        }
    }

    fn consume(&mut self, expected: &str) -> Result<()> {
        if self.starts_with(expected) {
            self.position += expected.len();
            return Ok(());
        }
        Err(self.error(format!("expected `{expected}`")))
    }

    fn starts_with(&self, expected: &str) -> bool {
        self.input[self.position..].starts_with(expected)
    }

    fn current_byte(&self) -> Option<u8> {
        self.bytes.get(self.position).copied()
    }

    fn find_byte(&self, byte: u8) -> Option<usize> {
        memchr(byte, &self.bytes[self.position..]).map(|offset| self.position + offset)
    }

    const fn quote_byte(quote: QuoteStyle) -> u8 {
        match quote {
            QuoteStyle::Double => b'"',
            QuoteStyle::Single => b'\'',
        }
    }

    fn error(&self, message: impl Into<String>) -> FerroviaError {
        FerroviaError::Parse {
            position: self.position,
            message: message.into(),
        }
    }
}
