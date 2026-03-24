use crate::types::{XastChild, XastRoot};

const VIEWBOX_ELEMS: &[&str] = &["pattern", "svg", "symbol"];

/// Apply the `removeViewBox` plugin.
///
/// # Errors
///
/// This direct port currently does not return plugin-specific runtime errors.
pub fn apply(root: &mut XastRoot) -> crate::error::Result<()> {
    remove_view_box(&mut root.children, true);
    Ok(())
}

fn remove_view_box(children: &mut [XastChild], parent_is_root: bool) {
    for child in children {
        if let XastChild::Element(element) = child {
            if VIEWBOX_ELEMS.contains(&element.name.as_str())
                && element.get_attribute("viewBox").is_some()
                && element.get_attribute("width").is_some()
                && element.get_attribute("height").is_some()
            {
                let is_nested_svg = element.name == "svg" && !parent_is_root;
                if !is_nested_svg
                    && let (Some(view_box), Some(width), Some(height)) = (
                        element.get_attribute("viewBox"),
                        element.get_attribute("width"),
                        element.get_attribute("height"),
                    )
                {
                    let numbers = view_box
                        .split([' ', ','])
                        .filter(|part| !part.is_empty())
                        .collect::<Vec<_>>();
                    if numbers.len() == 4
                        && numbers[0] == "0"
                        && numbers[1] == "0"
                        && width.trim_end_matches("px") == numbers[2]
                        && height.trim_end_matches("px") == numbers[3]
                    {
                        element.remove_attribute("viewBox");
                    }
                }
            }
            remove_view_box(&mut element.children, false);
        }
    }
}
