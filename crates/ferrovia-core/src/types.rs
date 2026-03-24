use std::collections::HashMap;

use crate::config::Js2Svg;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XastDoctype {
    pub name: String,
    pub doctype: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XastInstruction {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XastComment {
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XastCdata {
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XastText {
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XastAttribute {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XastElement {
    pub name: String,
    pub attributes: Vec<XastAttribute>,
    pub children: Vec<XastChild>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XastChild {
    Doctype(XastDoctype),
    Instruction(XastInstruction),
    Comment(XastComment),
    Cdata(XastCdata),
    Text(XastText),
    Element(XastElement),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XastRoot {
    pub children: Vec<XastChild>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XastNode<'a> {
    Root(&'a XastRoot),
    Doctype(&'a XastDoctype),
    Instruction(&'a XastInstruction),
    Comment(&'a XastComment),
    Cdata(&'a XastCdata),
    Text(&'a XastText),
    Element(&'a XastElement),
}

pub type StringifyOptions = Js2Svg;

pub type AttributesMap = HashMap<String, String>;

impl XastRoot {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }
}

impl Default for XastRoot {
    fn default() -> Self {
        Self::new()
    }
}
