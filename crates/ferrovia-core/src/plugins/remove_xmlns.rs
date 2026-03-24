use crate::types::{XastChild, XastRoot};

/// Apply the `removeXMLNS` plugin.
///
/// # Errors
///
/// This direct port currently does not return plugin-specific runtime errors.
pub fn apply(root: &mut XastRoot) -> crate::error::Result<()> {
    remove_xmlns(&mut root.children);
    Ok(())
}

fn remove_xmlns(children: &mut [XastChild]) {
    for child in children {
        if let XastChild::Element(element) = child {
            if element.name == "svg" {
                element.remove_attribute("xmlns");
            }
            remove_xmlns(&mut element.children);
        }
    }
}
