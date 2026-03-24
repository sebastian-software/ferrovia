use ferrovia_core::plugins::remove_attrs;
use ferrovia_core::types::{XastAttribute, XastChild, XastElement, XastRoot};

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
fn removes_basic_attribute_pattern() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![XastChild::Element(element(
                "path",
                &[("fill", "red"), ("stroke", "blue")],
                Vec::new(),
            ))],
        ))],
    };

    let params = serde_json::json!({
        "attrs": "fill"
    });

    remove_attrs::apply(&mut root, Some(&params)).expect("apply plugin");
    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(path) = &svg.children[0] else {
        panic!("expected path");
    };
    assert_eq!(path.get_attribute("fill"), None);
    assert_eq!(path.get_attribute("stroke"), Some("blue"));
}

#[test]
fn removes_pattern_with_element_and_value_match() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![
                XastChild::Element(element(
                    "path",
                    &[("fill", "none"), ("stroke", "blue")],
                    Vec::new(),
                )),
                XastChild::Element(element(
                    "rect",
                    &[("fill", "none"), ("stroke", "green")],
                    Vec::new(),
                )),
            ],
        ))],
    };

    let params = serde_json::json!({
        "attrs": "path:fill:none"
    });

    remove_attrs::apply(&mut root, Some(&params)).expect("apply plugin");
    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(path) = &svg.children[0] else {
        panic!("expected path");
    };
    let XastChild::Element(rect) = &svg.children[1] else {
        panic!("expected rect");
    };
    assert_eq!(path.get_attribute("fill"), None);
    assert_eq!(rect.get_attribute("fill"), Some("none"));
}

#[test]
fn preserves_current_color_when_requested() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![XastChild::Element(element(
                "path",
                &[("fill", "currentColor"), ("stroke", "currentColor")],
                Vec::new(),
            ))],
        ))],
    };

    let params = serde_json::json!({
        "attrs": ["fill", "stroke"],
        "preserveCurrentColor": true
    });

    remove_attrs::apply(&mut root, Some(&params)).expect("apply plugin");
    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(path) = &svg.children[0] else {
        panic!("expected path");
    };
    assert_eq!(path.get_attribute("fill"), Some("currentColor"));
    assert_eq!(path.get_attribute("stroke"), Some("currentColor"));
}
