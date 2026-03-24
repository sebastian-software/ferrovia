use ferrovia_core::plugins::convert_ellipse_to_circle;
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
fn converts_equal_radii_ellipse_to_circle() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "ellipse",
            &[("cx", "10"), ("cy", "12"), ("rx", "5"), ("ry", "5")],
            Vec::new(),
        ))],
    };

    convert_ellipse_to_circle::apply(&mut root).expect("apply plugin");

    let XastChild::Element(circle) = &root.children[0] else {
        panic!("expected circle");
    };
    assert_eq!(circle.name, "circle");
    assert_eq!(circle.get_attribute("r"), Some("5"));
    assert_eq!(circle.get_attribute("rx"), None);
    assert_eq!(circle.get_attribute("ry"), None);
}

#[test]
fn converts_svg2_auto_radii_to_circle() {
    let mut root = XastRoot {
        children: vec![
            XastChild::Element(element(
                "ellipse",
                &[("rx", "auto"), ("ry", "4")],
                Vec::new(),
            )),
            XastChild::Element(element(
                "ellipse",
                &[("rx", "7"), ("ry", "auto")],
                Vec::new(),
            )),
        ],
    };

    convert_ellipse_to_circle::apply(&mut root).expect("apply plugin");

    let XastChild::Element(first) = &root.children[0] else {
        panic!("expected first circle");
    };
    assert_eq!(first.name, "circle");
    assert_eq!(first.get_attribute("r"), Some("4"));

    let XastChild::Element(second) = &root.children[1] else {
        panic!("expected second circle");
    };
    assert_eq!(second.name, "circle");
    assert_eq!(second.get_attribute("r"), Some("7"));
}

#[test]
fn preserves_eccentric_ellipse() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "ellipse",
            &[("rx", "5"), ("ry", "6")],
            Vec::new(),
        ))],
    };

    convert_ellipse_to_circle::apply(&mut root).expect("apply plugin");

    let XastChild::Element(ellipse) = &root.children[0] else {
        panic!("expected ellipse");
    };
    assert_eq!(ellipse.name, "ellipse");
    assert_eq!(ellipse.get_attribute("rx"), Some("5"));
    assert_eq!(ellipse.get_attribute("ry"), Some("6"));
    assert_eq!(ellipse.get_attribute("r"), None);
}
