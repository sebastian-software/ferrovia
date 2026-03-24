use crate::types::{XastChild, XastElement, XastRoot};
use crate::util::map_nodes_to_parents::map_nodes_to_parents;

pub struct CssSelectAdapter<'a> {
    root: &'a XastRoot,
    parents: std::collections::HashMap<*const XastChild, *const XastElement>,
}

impl<'a> CssSelectAdapter<'a> {
    #[must_use]
    pub fn new(root: &'a XastRoot) -> Self {
        Self {
            root,
            parents: map_nodes_to_parents(root),
        }
    }

    #[must_use]
    pub const fn is_tag(node: &XastChild) -> bool {
        matches!(node, XastChild::Element(_))
    }

    #[must_use]
    pub const fn get_children(node: &XastChild) -> &[XastChild] {
        match node {
            XastChild::Element(element) => element.children.as_slice(),
            _ => &[],
        }
    }

    #[must_use]
    pub fn get_attribute_value<'b>(element: &'b XastElement, name: &str) -> Option<&'b str> {
        element.get_attribute(name)
    }

    #[must_use]
    pub const fn get_name(element: &'a XastElement) -> &'a str {
        element.name.as_str()
    }

    #[must_use]
    pub fn has_attrib(element: &XastElement, name: &str) -> bool {
        element.get_attribute(name).is_some()
    }

    #[must_use]
    pub fn get_text(element: &XastElement) -> &str {
        match element.children.first() {
            Some(XastChild::Text(text)) => text.value.as_str(),
            Some(XastChild::Cdata(cdata)) => cdata.value.as_str(),
            _ => "",
        }
    }

    #[must_use]
    pub fn get_parent(&self, node: &XastChild) -> Option<&'a XastElement> {
        let pointer = std::ptr::from_ref(node);
        let parent_pointer = *self.parents.get(&pointer)?;
        find_element_by_ptr(self.root, parent_pointer)
    }

    #[must_use]
    pub fn get_siblings(&self, node: &XastChild) -> &'a [XastChild] {
        self.get_parent(node).map_or_else(
            || self.root.children.as_slice(),
            |parent| parent.children.as_slice(),
        )
    }
}

fn find_element_by_ptr(root: &XastRoot, target: *const XastElement) -> Option<&XastElement> {
    for child in &root.children {
        if let XastChild::Element(element) = child {
            let pointer = std::ptr::from_ref(element);
            if pointer == target {
                return Some(element);
            }
            if let Some(found) = find_element_in_children(&element.children, target) {
                return Some(found);
            }
        }
    }
    None
}

fn find_element_in_children(
    children: &[XastChild],
    target: *const XastElement,
) -> Option<&XastElement> {
    for child in children {
        if let XastChild::Element(element) = child {
            let pointer = std::ptr::from_ref(element);
            if pointer == target {
                return Some(element);
            }
            if let Some(found) = find_element_in_children(&element.children, target) {
                return Some(found);
            }
        }
    }
    None
}
