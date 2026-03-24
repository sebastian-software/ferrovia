use ferrovia_css_select_compat::{Adapter, is_match, select_all, select_one};
use ferrovia_css_what_compat::{CompoundSelector, parse};

use crate::svgo::css_select_adapter::CssSelectAdapter;
use crate::types::{XastChild, XastElement, XastRoot};

pub fn detach_node_from_parent(children: &mut Vec<XastChild>, index: usize) {
    children.remove(index);
}

#[must_use]
pub fn query_selector_all<'a>(root: &'a XastRoot, selector: &str) -> Vec<&'a XastChild> {
    let selectors = parse(selector);
    if selectors.is_empty() {
        return Vec::new();
    }
    let adapter = CssSelectAdapter::new(root);
    select_all(&selectors, root.children.as_slice(), &adapter)
}

#[must_use]
pub fn query_selector<'a>(root: &'a XastRoot, selector: &str) -> Option<&'a XastChild> {
    let selectors = parse(selector);
    if selectors.is_empty() {
        return None;
    }
    let adapter = CssSelectAdapter::new(root);
    select_one(&selectors, root.children.as_slice(), &adapter)
}

#[must_use]
pub fn matches(node: &XastElement, selector: &str) -> bool {
    let selectors = parse(selector);
    if selectors.is_empty() {
        return false;
    }
    let adapter = DetachedElementAdapter;
    let detached = XastChild::Element(node.clone());
    is_match(&selectors, &detached, &[], &adapter)
}

struct DetachedElementAdapter;

impl<'a> Adapter<'a, XastChild> for DetachedElementAdapter {
    fn is_tag(&self, node: &XastChild) -> bool {
        matches!(node, XastChild::Element(_))
    }

    fn children(&self, node: &'a XastChild) -> &'a [XastChild] {
        match node {
            XastChild::Element(element) => element.children.as_slice(),
            _ => &[],
        }
    }

    fn matches_compound(&self, node: &XastChild, compound: &CompoundSelector) -> bool {
        let XastChild::Element(element) = node else {
            return false;
        };
        element_matches_compound(element, compound)
    }
}

pub(crate) fn element_matches_compound(element: &XastElement, compound: &CompoundSelector) -> bool {
    if !compound.universal
        && let Some(tag) = &compound.tag
        && element.name != *tag
    {
        return false;
    }

    if let Some(id) = &compound.id
        && element.get_attribute("id") != Some(id.as_str())
    {
        return false;
    }

    for class_name in &compound.classes {
        let Some(classes) = element.get_attribute("class") else {
            return false;
        };
        if !classes
            .split_ascii_whitespace()
            .any(|class| class == class_name)
        {
            return false;
        }
    }

    for attribute in &compound.attributes {
        let Some(value) = element.get_attribute(attribute.name.as_str()) else {
            return false;
        };
        if let Some(expected) = &attribute.value
            && value != expected
        {
            return false;
        }
    }

    true
}
