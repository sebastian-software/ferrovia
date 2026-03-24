#![allow(clippy::literal_string_with_formatting_args)]

use ferrovia_core::plugins::minify_styles;
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
fn minifies_style_element_text() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![XastChild::Element(element(
                "style",
                &[],
                vec![XastChild::Text(XastText {
                    value: " .a { fill : red ; } ".to_string(),
                })],
            ))],
        ))],
    };

    minify_styles::apply(&mut root, None).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(style) = &svg.children[0] else {
        panic!("expected style");
    };
    let XastChild::Text(text) = &style.children[0] else {
        panic!("expected text");
    };
    assert_eq!(text.value, ".a{fill:red}");
}

#[test]
fn removes_empty_style_element_after_minify() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![
                XastChild::Element(element(
                    "style",
                    &[],
                    vec![XastChild::Text(XastText {
                        value: "/* comment */".to_string(),
                    })],
                )),
                XastChild::Element(element("path", &[], Vec::new())),
            ],
        ))],
    };

    minify_styles::apply(&mut root, None).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(svg.children.len(), 1);
}

#[test]
fn minifies_style_attributes() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![XastChild::Element(element(
                "path",
                &[("style", " fill : red ; stroke : blue ; ")],
                Vec::new(),
            ))],
        ))],
    };

    minify_styles::apply(&mut root, None).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(path) = &svg.children[0] else {
        panic!("expected path");
    };
    assert_eq!(path.get_attribute("style"), Some("fill:red;stroke:blue"));
}

#[test]
fn keeps_cdata_when_original_style_contains_angle_brackets() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![XastChild::Element(element(
                "style",
                &[],
                vec![XastChild::Text(XastText {
                    value: "a::before { content: \"<\"; }".to_string(),
                })],
            ))],
        ))],
    };

    minify_styles::apply(&mut root, Some(&json!({}))).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(style) = &svg.children[0] else {
        panic!("expected style");
    };
    assert!(matches!(style.children[0], XastChild::Cdata(_)));
}

