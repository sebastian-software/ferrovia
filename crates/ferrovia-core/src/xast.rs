use crate::types::{XastChild, XastElement, XastRoot};

pub fn detach_node_from_parent(children: &mut Vec<XastChild>, index: usize) {
    children.remove(index);
}

#[must_use]
pub const fn query_selector_all<'a>(_root: &'a XastRoot, _selector: &str) -> Vec<&'a XastChild> {
    Vec::new()
}

#[must_use]
pub const fn query_selector<'a>(_root: &'a XastRoot, _selector: &str) -> Option<&'a XastChild> {
    None
}

#[must_use]
pub const fn matches(_node: &XastElement, _selector: &str) -> bool {
    false
}
