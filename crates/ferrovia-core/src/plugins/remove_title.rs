use crate::types::{XastChild, XastRoot};

/// Apply the `removeTitle` plugin.
///
/// # Errors
///
/// This direct port currently does not return plugin-specific runtime errors.
pub fn apply(root: &mut XastRoot) -> crate::error::Result<()> {
    remove_named_elements(&mut root.children, "title");
    Ok(())
}

fn remove_named_elements(children: &mut Vec<XastChild>, name: &str) {
    let mut index = 0usize;
    while index < children.len() {
        let mut removed = false;
        match &mut children[index] {
            XastChild::Element(element) if element.name == name => {
                children.remove(index);
                removed = true;
            }
            XastChild::Element(element) => remove_named_elements(&mut element.children, name),
            _ => {}
        }
        if !removed {
            index += 1;
        }
    }
}
