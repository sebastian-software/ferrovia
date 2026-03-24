use ferrovia_core::plugins::cleanup_attrs;
use ferrovia_core::types::{XastAttribute, XastChild, XastElement, XastRoot};
use serde_json::json;

fn element(name: &str, attributes: &[(&str, &str)], children: Vec<XastChild>) -> XastElement {
    XastElement {
        name: name.to_string(),
        attributes: attributes
            .iter()
            .map(|(name, value)| XastAttribute {
                name: (*name).to_string(),
                value: (*value).to_string(),
            })
            .collect(),
        children,
    }
}

#[test]
fn cleans_newlines_trim_and_repeated_spaces_by_default() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[("data-name", "  foo\nbar  baz \n qux  ")],
            Vec::new(),
        ))],
    };

    cleanup_attrs::apply(&mut root, None).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(svg.get_attribute("data-name"), Some("foo bar baz qux"));
}

#[test]
fn can_disable_individual_cleanup_steps() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[("data-name", "  foo\nbar  baz  ")],
            Vec::new(),
        ))],
    };

    cleanup_attrs::apply(
        &mut root,
        Some(&json!({
            "newlines": false,
            "trim": false,
            "spaces": false,
        })),
    )
    .expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(svg.get_attribute("data-name"), Some("  foo\nbar  baz  "));
}

#[test]
fn traverses_nested_elements() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![XastChild::Element(element(
                "g",
                &[("data-name", "foo \n bar")],
                Vec::new(),
            ))],
        ))],
    };

    cleanup_attrs::apply(&mut root, None).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(group) = &svg.children[0] else {
        panic!("expected group");
    };
    assert_eq!(group.get_attribute("data-name"), Some("foo bar"));
}
