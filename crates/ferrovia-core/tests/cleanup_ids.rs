#![allow(clippy::literal_string_with_formatting_args)]

use ferrovia_core::plugins::cleanup_ids;
use ferrovia_core::types::{XastAttribute, XastChild, XastElement, XastRoot, XastText};
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
fn minifies_referenced_ids_and_rewrites_references() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![
                XastChild::Element(element("linearGradient", &[("id", "paint0")], Vec::new())),
                XastChild::Element(element("path", &[("fill", "url(#paint0)")], Vec::new())),
                XastChild::Element(element("use", &[("href", "#paint0")], Vec::new())),
                XastChild::Element(element(
                    "animate",
                    &[("begin", "paint0.begin+1s")],
                    Vec::new(),
                )),
            ],
        ))],
    };

    cleanup_ids::apply(&mut root, None).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(gradient) = &svg.children[0] else {
        panic!("expected gradient");
    };
    assert_eq!(gradient.get_attribute("id"), Some("a"));

    let XastChild::Element(path) = &svg.children[1] else {
        panic!("expected path");
    };
    assert_eq!(path.get_attribute("fill"), Some("url(#a)"));

    let XastChild::Element(use_node) = &svg.children[2] else {
        panic!("expected use");
    };
    assert_eq!(use_node.get_attribute("href"), Some("#a"));

    let XastChild::Element(animate) = &svg.children[3] else {
        panic!("expected animate");
    };
    assert_eq!(animate.get_attribute("begin"), Some("a.begin+1s"));
}

#[test]
fn removes_duplicate_and_unused_ids() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![
                XastChild::Element(element("g", &[("id", "dup")], Vec::new())),
                XastChild::Element(element("g", &[("id", "dup")], Vec::new())),
                XastChild::Element(element("path", &[("id", "unused")], Vec::new())),
            ],
        ))],
    };

    cleanup_ids::apply(&mut root, None).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(first) = &svg.children[0] else {
        panic!("expected first");
    };
    let XastChild::Element(second) = &svg.children[1] else {
        panic!("expected second");
    };
    let XastChild::Element(third) = &svg.children[2] else {
        panic!("expected third");
    };
    assert_eq!(first.get_attribute("id"), None);
    assert_eq!(second.get_attribute("id"), None);
    assert_eq!(third.get_attribute("id"), None);
}

#[test]
fn respects_preserve_and_deoptimizes_on_style() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![
                XastChild::Element(element(
                    "style",
                    &[],
                    vec![XastChild::Text(XastText {
                        value: "#keep{fill:red}".to_string(),
                    })],
                )),
                XastChild::Element(element("path", &[("id", "keep")], Vec::new())),
            ],
        ))],
    };

    cleanup_ids::apply(
        &mut root,
        Some(&json!({
            "preserve": ["keep"],
        })),
    )
    .expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(path) = &svg.children[1] else {
        panic!("expected path");
    };
    assert_eq!(path.get_attribute("id"), Some("keep"));
}
