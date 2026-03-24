#![allow(clippy::literal_string_with_formatting_args)]

use ferrovia_core::plugins::_path::{intersects, js2path, path2js};
use ferrovia_core::plugins::_transforms::{js2transform, transform2js, transforms_multiply};
use ferrovia_core::plugins::apply_transforms::{ApplyTransformsParams, apply_transforms};
use ferrovia_core::style::{collect_stylesheet, compute_style};
use ferrovia_core::types::{
    Js2PathParams, TransformParams, XastAttribute, XastChild, XastElement, XastRoot, XastText,
};

fn path_element(d: &str) -> XastElement {
    XastElement {
        name: "path".to_string(),
        attributes: vec![XastAttribute {
            name: "d".to_string(),
            value: d.to_string(),
        }],
        children: Vec::new(),
    }
}

#[test]
fn path_helper_promotes_initial_relative_moveto() {
    let path = path_element("m0 0 10 10");
    let data = path2js(&path);
    assert_eq!(data[0].command, 'M');
    assert_eq!(data[1].command, 'l');
}

#[test]
fn js2path_replaces_redundant_leading_moveto() {
    let mut path = path_element("M0 0");
    let mut data = path2js(&path);
    data.insert(
        0,
        ferrovia_core::types::PathDataItem {
            command: 'M',
            args: vec![5.0, 5.0],
        },
    );
    js2path(&mut path, &data, &Js2PathParams::default());
    assert_eq!(path.get_attribute("d"), Some("M0 0"));
}

#[test]
fn transform_helpers_parse_and_stringify_basic_transforms() {
    let parsed = transform2js("translate(10 20)scale(2)");
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].name, "translate");
    let matrix = transforms_multiply(&parsed);
    assert_eq!(matrix.name, "matrix");
    assert_eq!(
        js2transform(&parsed, &TransformParams::default()),
        "translate(10 20)scale(2)"
    );
}

#[test]
fn apply_transforms_rewrites_simple_path_transform() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(XastElement {
            name: "svg".to_string(),
            attributes: Vec::new(),
            children: vec![XastChild::Element(XastElement {
                name: "path".to_string(),
                attributes: vec![
                    XastAttribute {
                        name: "d".to_string(),
                        value: "M0 0L10 0".to_string(),
                    },
                    XastAttribute {
                        name: "transform".to_string(),
                        value: "translate(5 0)".to_string(),
                    },
                ],
                children: Vec::new(),
            })],
        })],
    };

    apply_transforms(&mut root, &ApplyTransformsParams::default());
    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(path) = &svg.children[0] else {
        panic!("expected path");
    };
    assert_eq!(path.get_attribute("transform"), None);
    assert_eq!(path.get_attribute("d"), Some("M5 0L15 0"));
}

#[test]
fn computes_style_for_tag_class_and_inline_sources() {
    let root = XastRoot {
        children: vec![XastChild::Element(XastElement {
            name: "svg".to_string(),
            attributes: Vec::new(),
            children: vec![
                XastChild::Element(XastElement {
                    name: "style".to_string(),
                    attributes: Vec::new(),
                    children: vec![XastChild::Text(XastText {
                        value: r"path{fill:red}.hero{stroke:blue}".to_string(),
                    })],
                }),
                XastChild::Element(XastElement {
                    name: "path".to_string(),
                    attributes: vec![
                        XastAttribute {
                            name: "class".to_string(),
                            value: "hero".to_string(),
                        },
                        XastAttribute {
                            name: "style".to_string(),
                            value: "transform:translate(10 0)".to_string(),
                        },
                    ],
                    children: Vec::new(),
                }),
            ],
        })],
    };

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let stylesheet = collect_stylesheet(&root);
    let XastChild::Element(path) = &svg.children[1] else {
        panic!("expected path");
    };
    let computed = compute_style(&stylesheet, path);
    assert!(computed.iter().any(|(name, _)| name == "fill"));
    assert!(computed.iter().any(|(name, _)| name == "stroke"));
    assert!(computed.iter().any(|(name, _)| name == "transform"));
}

#[test]
fn path_intersection_helper_uses_conservative_bounds() {
    let left = path2js(&path_element("M0 0L10 0L10 10z"));
    let right = path2js(&path_element("M5 5L15 5L15 15z"));
    let far = path2js(&path_element("M20 20L30 20L30 30z"));
    assert!(intersects(&left, &right));
    assert!(!intersects(&left, &far));
}
