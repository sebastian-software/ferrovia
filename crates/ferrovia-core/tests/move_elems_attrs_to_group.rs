#![allow(clippy::literal_string_with_formatting_args)]

use ferrovia_core::plugins::move_elems_attrs_to_group;
use ferrovia_core::types::{XastAttribute, XastChild, XastElement, XastRoot, XastText};

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
fn moves_common_inheritable_attrs_to_group() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![XastChild::Element(element(
                "g",
                &[],
                vec![
                    XastChild::Element(element("circle", &[("fill", "red")], Vec::new())),
                    XastChild::Element(element("rect", &[("fill", "red")], Vec::new())),
                ],
            ))],
        ))],
    };

    move_elems_attrs_to_group::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(group) = &svg.children[0] else {
        panic!("expected group");
    };
    assert_eq!(group.get_attribute("fill"), Some("red"));

    let XastChild::Element(first) = &group.children[0] else {
        panic!("expected first child");
    };
    let XastChild::Element(second) = &group.children[1] else {
        panic!("expected second child");
    };
    assert_eq!(first.get_attribute("fill"), None);
    assert_eq!(second.get_attribute("fill"), None);
}

#[test]
fn preserves_transform_on_all_path_children() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![XastChild::Element(element(
                "g",
                &[],
                vec![
                    XastChild::Element(element("path", &[("transform", "scale(2)")], Vec::new())),
                    XastChild::Element(element("path", &[("transform", "scale(2)")], Vec::new())),
                ],
            ))],
        ))],
    };

    move_elems_attrs_to_group::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(group) = &svg.children[0] else {
        panic!("expected group");
    };
    assert_eq!(group.get_attribute("transform"), None);
}

#[test]
fn preserves_transform_when_group_has_filter_like_attrs() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![XastChild::Element(element(
                "g",
                &[("filter", "url(#f)")],
                vec![
                    XastChild::Element(element("circle", &[("transform", "scale(2)")], Vec::new())),
                    XastChild::Element(element("rect", &[("transform", "scale(2)")], Vec::new())),
                ],
            ))],
        ))],
    };

    move_elems_attrs_to_group::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(group) = &svg.children[0] else {
        panic!("expected group");
    };
    assert_eq!(group.get_attribute("transform"), None);
}

#[test]
fn deoptimizes_when_style_element_exists() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![
                XastChild::Element(element(
                    "style",
                    &[],
                    vec![XastChild::Text(XastText {
                        value: ".hero{fill:red}".to_string(),
                    })],
                )),
                XastChild::Element(element(
                    "g",
                    &[],
                    vec![
                        XastChild::Element(element("circle", &[("fill", "red")], Vec::new())),
                        XastChild::Element(element("rect", &[("fill", "red")], Vec::new())),
                    ],
                )),
            ],
        ))],
    };

    move_elems_attrs_to_group::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(group) = &svg.children[1] else {
        panic!("expected group");
    };
    assert_eq!(group.get_attribute("fill"), None);
}
