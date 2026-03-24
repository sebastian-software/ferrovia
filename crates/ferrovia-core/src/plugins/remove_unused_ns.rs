use std::collections::HashSet;

use crate::types::{XastChild, XastElement, XastRoot};

/// Apply the `removeUnusedNS` plugin.
///
/// # Errors
///
/// This direct port currently does not return plugin-specific runtime errors.
pub fn apply(root: &mut XastRoot) -> crate::error::Result<()> {
    let mut unused_namespaces = HashSet::<String>::new();
    collect_unused_namespaces(root, &mut unused_namespaces);
    if unused_namespaces.is_empty() {
        return Ok(());
    }
    mark_used_namespaces(&root.children, &mut unused_namespaces);
    remove_unused_namespace_attributes(root, &unused_namespaces);
    Ok(())
}

fn collect_unused_namespaces(root: &XastRoot, unused_namespaces: &mut HashSet<String>) {
    if let Some(XastChild::Element(svg)) = root.children.first()
        && svg.name == "svg"
    {
        for attribute in &svg.attributes {
            if let Some(local) = attribute.name.strip_prefix("xmlns:") {
                unused_namespaces.insert(local.to_string());
            }
        }
    }
}

fn mark_used_namespaces(children: &[XastChild], unused_namespaces: &mut HashSet<String>) {
    for child in children {
        if let XastChild::Element(element) = child {
            if let Some((namespace, _)) = element.name.split_once(':') {
                unused_namespaces.remove(namespace);
            }
            for attribute in &element.attributes {
                if let Some((namespace, _)) = attribute.name.split_once(':') {
                    unused_namespaces.remove(namespace);
                }
            }
            if !unused_namespaces.is_empty() {
                mark_used_namespaces(&element.children, unused_namespaces);
            }
        }
    }
}

fn remove_unused_namespace_attributes(root: &mut XastRoot, unused_namespaces: &HashSet<String>) {
    let Some(XastChild::Element(svg)) = root.children.first_mut() else {
        return;
    };
    if svg.name != "svg" {
        return;
    }
    svg.attributes.retain(|attribute| {
        attribute
            .name
            .strip_prefix("xmlns:")
            .is_none_or(|namespace| !unused_namespaces.contains(namespace))
    });
}

#[allow(dead_code)]
fn _root_svg(root: &mut XastRoot) -> Option<&mut XastElement> {
    let Some(XastChild::Element(svg)) = root.children.first_mut() else {
        return None;
    };
    (svg.name == "svg").then_some(svg)
}
