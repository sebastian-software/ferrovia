use crate::types::{XastChild, XastRoot};

/// Apply the `removeDimensions` plugin.
///
/// # Errors
///
/// This direct port currently does not return plugin-specific runtime errors.
pub fn apply(root: &mut XastRoot) -> crate::error::Result<()> {
    remove_dimensions(&mut root.children);
    Ok(())
}

fn remove_dimensions(children: &mut [XastChild]) {
    for child in children {
        if let XastChild::Element(element) = child {
            if element.name == "svg" {
                let view_box = element.get_attribute("viewBox").map(str::to_string);
                let width = element.get_attribute("width").map(str::to_string);
                let height = element.get_attribute("height").map(str::to_string);
                if view_box.is_some() {
                    element.remove_attribute("width");
                    element.remove_attribute("height");
                } else if let (Some(width), Some(height)) = (width, height)
                    && let (Ok(width), Ok(height)) = (width.parse::<f64>(), height.parse::<f64>())
                {
                    element.set_attribute("viewBox", format!("0 0 {width} {height}"));
                    element.remove_attribute("width");
                    element.remove_attribute("height");
                }
            }
            remove_dimensions(&mut element.children);
        }
    }
}
