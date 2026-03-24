use crate::types::{XastChild, XastRoot};

/// Apply the `removeDoctype` plugin.
///
/// # Errors
///
/// This direct port currently does not return plugin-specific runtime errors.
pub fn apply(root: &mut XastRoot) -> crate::error::Result<()> {
    root.children.retain(|child| !matches!(child, XastChild::Doctype(_)));
    Ok(())
}
