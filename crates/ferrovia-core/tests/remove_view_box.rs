use ferrovia_core::plugins::remove_view_box;
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
fn removes_matching_root_svg_viewbox() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[
                ("width", "100"),
                ("height", "50"),
                ("viewBox", "0 0 100 50"),
            ],
            Vec::new(),
        ))],
    };

    remove_view_box::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(svg.get_attribute("viewBox"), None);
}

#[test]
fn preserves_nested_svg_viewbox() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[
                ("width", "100"),
                ("height", "50"),
                ("viewBox", "0 0 100 50"),
            ],
            vec![XastChild::Element(element(
                "svg",
                &[("width", "10"), ("height", "10"), ("viewBox", "0 0 10 10")],
                Vec::new(),
            ))],
        ))],
    };

    remove_view_box::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(nested) = &svg.children[0] else {
        panic!("expected nested svg");
    };
    assert_eq!(nested.get_attribute("viewBox"), Some("0 0 10 10"));
}

#[test]
fn preserves_non_matching_viewbox() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "symbol",
            &[
                ("width", "100"),
                ("height", "50"),
                ("viewBox", "10 0 100 50"),
            ],
            Vec::new(),
        ))],
    };

    remove_view_box::apply(&mut root).expect("apply plugin");

    let XastChild::Element(symbol) = &root.children[0] else {
        panic!("expected symbol");
    };
    assert_eq!(symbol.get_attribute("viewBox"), Some("10 0 100 50"));
}
