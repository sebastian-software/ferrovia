use ferrovia_core::plugins::remove_unknowns_and_defaults;
use ferrovia_core::types::{XastAttribute, XastChild, XastElement, XastInstruction, XastRoot};

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
fn removes_default_xml_standalone_declaration() {
    let mut root = XastRoot {
        children: vec![
            XastChild::Instruction(XastInstruction {
                name: "xml".to_string(),
                value: r#"version="1.0" standalone="no""#.to_string(),
            }),
            XastChild::Element(element("svg", &[("xmlns", "http://www.w3.org/2000/svg")], Vec::new())),
        ],
    };

    remove_unknowns_and_defaults::apply(&mut root, None).expect("apply plugin");

    let XastChild::Instruction(instruction) = &root.children[0] else {
        panic!("expected xml instruction");
    };
    assert_eq!(instruction.value, r#"version="1.0""#);
}

#[test]
fn removes_unknown_children_and_unknown_attributes() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[("xmlns", "http://www.w3.org/2000/svg")],
            vec![
                XastChild::Element(element("bogus", &[], Vec::new())),
                XastChild::Element(element("path", &[("d", "M0 0"), ("bogus", "1")], Vec::new())),
            ],
        ))],
    };

    remove_unknowns_and_defaults::apply(&mut root, None).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(svg.children.len(), 1);
    let XastChild::Element(path) = &svg.children[0] else {
        panic!("expected path");
    };
    assert_eq!(path.get_attribute("bogus"), None);
}

#[test]
fn removes_default_attrs_without_id() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[("xmlns", "http://www.w3.org/2000/svg")],
            vec![XastChild::Element(element(
                "path",
                &[("d", "M0 0"), ("fill", "#000"), ("stroke", "none")],
                Vec::new(),
            ))],
        ))],
    };

    remove_unknowns_and_defaults::apply(&mut root, None).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(path) = &svg.children[0] else {
        panic!("expected path");
    };
    assert_eq!(path.get_attribute("fill"), None);
    assert_eq!(path.get_attribute("stroke"), None);
}

#[test]
fn preserves_default_attrs_when_element_has_id() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[("xmlns", "http://www.w3.org/2000/svg")],
            vec![XastChild::Element(element(
                "path",
                &[("id", "p"), ("d", "M0 0"), ("fill", "#000")],
                Vec::new(),
            ))],
        ))],
    };

    remove_unknowns_and_defaults::apply(&mut root, None).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(path) = &svg.children[0] else {
        panic!("expected path");
    };
    assert_eq!(path.get_attribute("fill"), Some("#000"));
}

#[test]
fn removes_useless_inheritable_overrides() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[("xmlns", "http://www.w3.org/2000/svg")],
            vec![XastChild::Element(element(
                "g",
                &[("fill", "red")],
                vec![XastChild::Element(element(
                    "path",
                    &[("d", "M0 0"), ("fill", "red"), ("opacity", "1")],
                    Vec::new(),
                ))],
            ))],
        ))],
    };

    remove_unknowns_and_defaults::apply(&mut root, None).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(group) = &svg.children[0] else {
        panic!("expected group");
    };
    let XastChild::Element(path) = &group.children[0] else {
        panic!("expected path");
    };
    assert_eq!(path.get_attribute("fill"), None);
    assert_eq!(path.get_attribute("opacity"), None);
}

