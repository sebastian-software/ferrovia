use crate::types::{XastChild, XastRoot};

/// Apply the `convertEllipseToCircle` plugin.
///
/// # Errors
///
/// This direct port currently does not return plugin-specific runtime errors.
pub fn apply(root: &mut XastRoot) -> crate::error::Result<()> {
    convert_ellipse_to_circle(&mut root.children);
    Ok(())
}

fn convert_ellipse_to_circle(children: &mut [XastChild]) {
    for child in children {
        if let XastChild::Element(element) = child {
            if element.name == "ellipse" {
                let rx = element.get_attribute("rx").unwrap_or("0");
                let ry = element.get_attribute("ry").unwrap_or("0");
                if rx == ry || rx == "auto" || ry == "auto" {
                    let radius = if rx == "auto" { ry } else { rx }.to_string();
                    element.name = "circle".to_string();
                    element.remove_attribute("rx");
                    element.remove_attribute("ry");
                    element.set_attribute("r", radius);
                }
            }
            convert_ellipse_to_circle(&mut element.children);
        }
    }
}
