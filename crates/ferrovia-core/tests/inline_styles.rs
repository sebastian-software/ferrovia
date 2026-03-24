#![allow(clippy::literal_string_with_formatting_args)]

use ferrovia_core::plugins::inline_styles;
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
fn inlines_simple_rule_and_removes_matched_selector() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![
                XastChild::Element(element(
                    "style",
                    &[],
                    vec![XastChild::Text(XastText {
                        value: ".a{fill:red}".to_string(),
                    })],
                )),
                XastChild::Element(element("path", &[("class", "a")], Vec::new())),
            ],
        ))],
    };

    inline_styles::apply(&mut root, None).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(svg.children.len(), 1);

    let XastChild::Element(path) = &svg.children[0] else {
        panic!("expected path");
    };
    assert_eq!(path.get_attribute("style"), Some("fill:red"));
    assert_eq!(path.get_attribute("class"), None);
}

#[test]
fn respects_only_matched_once_by_default() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![
                XastChild::Element(element(
                    "style",
                    &[],
                    vec![XastChild::Text(XastText {
                        value: ".a{fill:red}".to_string(),
                    })],
                )),
                XastChild::Element(element("path", &[("class", "a")], Vec::new())),
                XastChild::Element(element("rect", &[("class", "a")], Vec::new())),
            ],
        ))],
    };

    inline_styles::apply(&mut root, None).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(svg.children.len(), 3);

    let XastChild::Element(path) = &svg.children[1] else {
        panic!("expected path");
    };
    let XastChild::Element(rect) = &svg.children[2] else {
        panic!("expected rect");
    };
    assert_eq!(path.get_attribute("style"), None);
    assert_eq!(rect.get_attribute("style"), None);
}

#[test]
fn can_inline_multi_match_when_option_disabled() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![
                XastChild::Element(element(
                    "style",
                    &[],
                    vec![XastChild::Text(XastText {
                        value: ".a{fill:red}".to_string(),
                    })],
                )),
                XastChild::Element(element("path", &[("class", "a")], Vec::new())),
                XastChild::Element(element("rect", &[("class", "a")], Vec::new())),
            ],
        ))],
    };

    let params = json!({
        "onlyMatchedOnce": false
    });
    inline_styles::apply(&mut root, Some(&params)).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(svg.children.len(), 2);

    let XastChild::Element(path) = &svg.children[0] else {
        panic!("expected path");
    };
    let XastChild::Element(rect) = &svg.children[1] else {
        panic!("expected rect");
    };
    assert_eq!(path.get_attribute("style"), Some("fill:red"));
    assert_eq!(rect.get_attribute("style"), Some("fill:red"));
}

#[test]
fn keeps_foreign_object_styles_untouched() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![XastChild::Element(element(
                "foreignObject",
                &[],
                vec![
                    XastChild::Element(element(
                        "style",
                        &[],
                        vec![XastChild::Text(XastText {
                            value: ".a{fill:red}".to_string(),
                        })],
                    )),
                    XastChild::Element(element("div", &[("class", "a")], Vec::new())),
                ],
            ))],
        ))],
    };

    inline_styles::apply(&mut root, None).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(foreign_object) = &svg.children[0] else {
        panic!("expected foreignObject");
    };
    let XastChild::Element(style) = &foreign_object.children[0] else {
        panic!("expected style");
    };
    let XastChild::Text(text) = &style.children[0] else {
        panic!("expected text");
    };
    assert_eq!(text.value, ".a{fill:red}");
}
