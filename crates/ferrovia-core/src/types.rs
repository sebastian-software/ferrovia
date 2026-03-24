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

#[derive(Debug, Clone, PartialEq)]
pub struct TransformItem {
    pub name: String,
    pub data: Vec<f64>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Js2PathParams {
    pub float_precision: Option<usize>,
    pub no_space_after_flags: bool,
}

impl Default for Js2PathParams {
    fn default() -> Self {
        Self {
            float_precision: Some(3),
            no_space_after_flags: false,
        }
    }
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransformParams {
    pub convert_to_shorts: bool,
    pub deg_precision: Option<usize>,
    pub float_precision: usize,
    pub transform_precision: usize,
    pub matrix_to_transform: bool,
    pub short_translate: bool,
    pub short_scale: bool,
    pub short_rotate: bool,
    pub remove_useless: bool,
    pub collapse_into_one: bool,
    pub leading_zero: bool,
    pub negative_extra_space: bool,
}

impl Default for TransformParams {
    fn default() -> Self {
        Self {
            convert_to_shorts: true,
            deg_precision: Some(3),
            float_precision: 3,
            transform_precision: 5,
            matrix_to_transform: true,
            short_translate: true,
            short_scale: true,
            short_rotate: true,
            remove_useless: true,
            collapse_into_one: true,
            leading_zero: true,
            negative_extra_space: true,
        }
    }
}

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

impl XastElement {
    #[must_use]
    pub fn get_attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|attribute| attribute.name == name)
            .map(|attribute| attribute.value.as_str())
    }

    pub fn set_attribute(&mut self, name: &str, value: String) {
        if let Some(attribute) = self
            .attributes
            .iter_mut()
            .find(|attribute| attribute.name == name)
        {
            attribute.value = value;
            return;
        }
        self.attributes.push(XastAttribute {
            name: name.to_string(),
            value,
        });
    }

    pub fn remove_attribute(&mut self, name: &str) -> Option<String> {
        self.attributes
            .iter()
            .position(|attribute| attribute.name == name)
            .map(|index| self.attributes.remove(index).value)
    }
}
