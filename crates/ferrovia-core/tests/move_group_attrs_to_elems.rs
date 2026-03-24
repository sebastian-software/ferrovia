use ferrovia_core::plugins::move_group_attrs_to_elems;
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
fn moves_group_transform_to_supported_children() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![XastChild::Element(element(
                "g",
                &[("transform", "scale(2)")],
                vec![
                    XastChild::Element(element("path", &[("d", "M0 0")], Vec::new())),
                    XastChild::Element(element("text", &[], Vec::new())),
                ],
            ))],
        ))],
    };

    move_group_attrs_to_elems::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(group) = &svg.children[0] else {
        panic!("expected group");
    };
    assert_eq!(group.get_attribute("transform"), None);

    let XastChild::Element(path) = &group.children[0] else {
        panic!("expected path");
    };
    assert_eq!(path.get_attribute("transform"), Some("scale(2)"));

    let XastChild::Element(text) = &group.children[1] else {
        panic!("expected text");
    };
    assert_eq!(text.get_attribute("transform"), Some("scale(2)"));
}

#[test]
fn prepends_group_transform_to_existing_child_transform() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![XastChild::Element(element(
                "g",
                &[("transform", "scale(2)")],
                vec![XastChild::Element(element(
                    "path",
                    &[("transform", "rotate(45)")],
                    Vec::new(),
                ))],
            ))],
        ))],
    };

    move_group_attrs_to_elems::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(group) = &svg.children[0] else {
        panic!("expected group");
    };
    let XastChild::Element(path) = &group.children[0] else {
        panic!("expected path");
    };
    assert_eq!(path.get_attribute("transform"), Some("scale(2) rotate(45)"));
}

#[test]
fn preserves_group_transform_when_group_references_url_props() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![XastChild::Element(element(
                "g",
                &[("transform", "scale(2)"), ("filter", "url(#f)")],
                vec![XastChild::Element(element("path", &[], Vec::new()))],
            ))],
        ))],
    };

    move_group_attrs_to_elems::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(group) = &svg.children[0] else {
        panic!("expected group");
    };
    assert_eq!(group.get_attribute("transform"), Some("scale(2)"));
}

#[test]
fn preserves_group_transform_when_child_has_id() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![XastChild::Element(element(
                "g",
                &[("transform", "scale(2)")],
                vec![XastChild::Element(element("path", &[("id", "p")], Vec::new()))],
            ))],
        ))],
    };

    move_group_attrs_to_elems::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(group) = &svg.children[0] else {
        panic!("expected group");
    };
    assert_eq!(group.get_attribute("transform"), Some("scale(2)"));
}
