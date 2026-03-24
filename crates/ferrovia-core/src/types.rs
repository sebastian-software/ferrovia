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

#[derive(Debug, Clone, PartialEq)]
pub struct PathDataItem {
    pub command: char,
    pub args: Vec<f64>,
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

pub type Specificity = [u32; 4];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StylesheetDeclaration {
    pub name: String,
    pub value: String,
    pub important: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StylesheetRule {
    pub selector: String,
    pub dynamic: bool,
    pub specificity: Specificity,
    pub declarations: Vec<StylesheetDeclaration>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComputedStyle {
    Static { inherited: bool, value: String },
    Dynamic { inherited: bool },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stylesheet {
    pub rules: Vec<StylesheetRule>,
}

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
