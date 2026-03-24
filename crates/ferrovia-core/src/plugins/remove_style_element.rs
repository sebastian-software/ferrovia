use crate::types::{XastChild, XastRoot};

/// Apply the `removeStyleElement` plugin.
///
/// # Errors
///
/// This direct port currently does not return plugin-specific runtime errors.
pub fn apply(root: &mut XastRoot) -> crate::error::Result<()> {
    remove_style_elements(&mut root.children);
    Ok(())
}

fn remove_style_elements(children: &mut Vec<XastChild>) {
    let mut index = 0usize;
    while index < children.len() {
        let mut removed = false;
        match &mut children[index] {
            XastChild::Element(element) if element.name == "style" => {
                children.remove(index);
                removed = true;
            }
            XastChild::Element(element) => remove_style_elements(&mut element.children),
            _ => {}
        }
        if !removed {
            index += 1;
        }
    }
}
