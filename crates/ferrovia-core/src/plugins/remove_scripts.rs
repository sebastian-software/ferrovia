use crate::plugins::_collections::is_event_attr;
use crate::types::{XastChild, XastRoot};

/// Apply the `removeScripts` plugin.
///
/// # Errors
///
/// This direct port currently does not return plugin-specific runtime errors.
pub fn apply(root: &mut XastRoot) -> crate::error::Result<()> {
    remove_scripts(&mut root.children);
    Ok(())
}

fn remove_scripts(children: &mut Vec<XastChild>) {
    let mut index = 0usize;
    while index < children.len() {
        let mut removed = false;
        match &mut children[index] {
            XastChild::Element(element) if element.name == "script" => {
                children.remove(index);
                removed = true;
            }
            XastChild::Element(element) => {
                element
                    .attributes
                    .retain(|attribute| !is_event_attr(attribute.name.as_str()));
                remove_scripts(&mut element.children);

                if element.name == "a"
                    && has_javascript_href(
                        element
                            .attributes
                            .iter()
                            .map(|attr| (attr.name.as_str(), attr.value.as_str())),
                    )
                {
                    let useful_children = std::mem::take(&mut element.children)
                        .into_iter()
                        .filter(|child| !matches!(child, XastChild::Text(_)))
                        .collect::<Vec<_>>();
                    children.remove(index);
                    for (offset, child) in useful_children.into_iter().enumerate() {
                        children.insert(index + offset, child);
                    }
                    removed = true;
                }
            }
            _ => {}
        }
        if !removed {
            index += 1;
        }
    }
}

fn has_javascript_href<'a>(mut attributes: impl Iterator<Item = (&'a str, &'a str)>) -> bool {
    attributes.any(|(name, value)| {
        (name == "href" || name.ends_with(":href")) && value.trim_start().starts_with("javascript:")
    })
}
