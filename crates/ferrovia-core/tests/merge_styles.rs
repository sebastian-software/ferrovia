#![allow(clippy::literal_string_with_formatting_args)]

use ferrovia_core::plugins::merge_styles;
use ferrovia_core::types::{XastAttribute, XastCdata, XastChild, XastElement, XastRoot, XastText};

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
fn merges_multiple_style_elements_into_the_first() {
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
                XastChild::Element(element(
                    "style",
                    &[],
                    vec![XastChild::Text(XastText {
                        value: ".b{fill:blue}".to_string(),
                    })],
                )),
            ],
        ))],
    };

    merge_styles::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(svg.children.len(), 1);

    let XastChild::Element(style) = &svg.children[0] else {
        panic!("expected style");
    };
    assert_eq!(style.name, "style");
    assert_eq!(style.children.len(), 1);
    let XastChild::Text(text) = &style.children[0] else {
        panic!("expected merged text");
    };
    assert_eq!(text.value, ".a{fill:red}.b{fill:blue}");
}

#[test]
fn wraps_media_styles_and_drops_media_attr_on_first_style() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![
                XastChild::Element(element(
                    "style",
                    &[("media", "screen")],
                    vec![XastChild::Text(XastText {
                        value: ".a{fill:red}".to_string(),
                    })],
                )),
                XastChild::Element(element(
                    "style",
                    &[],
                    vec![XastChild::Text(XastText {
                        value: ".b{fill:blue}".to_string(),
                    })],
                )),
            ],
        ))],
    };

    merge_styles::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(style) = &svg.children[0] else {
        panic!("expected style");
    };
    assert_eq!(style.get_attribute("media"), None);
    let XastChild::Text(text) = &style.children[0] else {
        panic!("expected text");
    };
    assert_eq!(text.value, "@media screen{.a{fill:red}}.b{fill:blue}");
}

#[test]
fn promotes_to_cdata_when_any_style_uses_cdata() {
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
                XastChild::Element(element(
                    "style",
                    &[],
                    vec![XastChild::Cdata(XastCdata {
                        value: ".b{fill:blue}".to_string(),
                    })],
                )),
            ],
        ))],
    };

    merge_styles::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(style) = &svg.children[0] else {
        panic!("expected style");
    };
    let XastChild::Cdata(cdata) = &style.children[0] else {
        panic!("expected cdata");
    };
    assert_eq!(cdata.value, ".a{fill:red}.b{fill:blue}");
}

#[test]
fn removes_empty_style_elements_and_skips_foreign_object_content() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![
                XastChild::Element(element("style", &[], vec![XastChild::Text(XastText {
                    value: " ".to_string(),
                })])),
                XastChild::Element(element(
                    "foreignObject",
                    &[],
                    vec![XastChild::Element(element(
                        "style",
                        &[],
                        vec![XastChild::Text(XastText {
                            value: ".x{display:none}".to_string(),
                        })],
                    ))],
                )),
                XastChild::Element(element(
                    "style",
                    &[("type", "application/ld+json")],
                    vec![XastChild::Text(XastText {
                        value: "{}".to_string(),
                    })],
                )),
            ],
        ))],
    };

    merge_styles::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(svg.children.len(), 2);
    let XastChild::Element(foreign_object) = &svg.children[0] else {
        panic!("expected foreignObject");
    };
    assert_eq!(foreign_object.name, "foreignObject");
    let XastChild::Element(style) = &svg.children[1] else {
        panic!("expected style");
    };
    assert_eq!(style.get_attribute("type"), Some("application/ld+json"));
}
