use std::collections::HashMap;

use crate::types::{XastChild, XastElement, XastRoot};

#[must_use]
pub fn map_nodes_to_parents(root: &XastRoot) -> HashMap<*const XastChild, *const XastElement> {
    let mut parents = HashMap::new();
    map_children(&root.children, None, &mut parents);
    parents
}

fn map_children(
    children: &[XastChild],
    parent: Option<&XastElement>,
    parents: &mut HashMap<*const XastChild, *const XastElement>,
) {
    for child in children {
        if let Some(parent) = parent {
            parents.insert(std::ptr::from_ref(child), std::ptr::from_ref(parent));
        }
        if let XastChild::Element(element) = child {
            map_children(&element.children, Some(element), parents);
        }
    }
}
