use regex::Regex;

use crate::types::{XastChild, XastRoot};

/// Apply the `removeRasterImages` plugin.
///
/// # Errors
///
/// This direct port currently does not return plugin-specific runtime errors.
pub fn apply(root: &mut XastRoot) -> crate::error::Result<()> {
    remove_raster_images(&mut root.children);
    Ok(())
}

fn remove_raster_images(children: &mut Vec<XastChild>) {
    let raster_href = Regex::new(r"(\.|image/)(jpe?g|png|gif)").expect("valid raster image regex");
    remove_raster_images_with_regex(children, &raster_href);
}

fn remove_raster_images_with_regex(children: &mut Vec<XastChild>, raster_href: &Regex) {
    let mut index = 0usize;
    while index < children.len() {
        let mut removed = false;
        match &mut children[index] {
            XastChild::Element(element)
                if element.name == "image"
                    && element
                        .get_attribute("xlink:href")
                        .is_some_and(|href| raster_href.is_match(href)) =>
            {
                children.remove(index);
                removed = true;
            }
            XastChild::Element(element) => {
                remove_raster_images_with_regex(&mut element.children, raster_href);
            }
            _ => {}
        }
        if !removed {
            index += 1;
        }
    }
}
