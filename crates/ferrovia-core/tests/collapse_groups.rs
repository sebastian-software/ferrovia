use ferrovia_core::plugins::collapse_groups;
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
fn collapses_attrless_groups() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![XastChild::Element(element(
                "g",
                &[],
                vec![XastChild::Element(element("path", &[("d", "M0 0")], Vec::new()))],
            ))],
        ))],
    };

    collapse_groups::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(path) = &svg.children[0] else {
        panic!("expected path");
    };
    assert_eq!(path.name, "path");
    assert_eq!(path.get_attribute("d"), Some("M0 0"));
}

#[test]
fn moves_group_attributes_to_single_child_and_collapses() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![XastChild::Element(element(
                "g",
                &[("fill", "red"), ("transform", "scale(2)")],
                vec![XastChild::Element(element("path", &[("d", "M0 0")], Vec::new()))],
            ))],
        ))],
    };

    collapse_groups::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(path) = &svg.children[0] else {
        panic!("expected path");
    };
    assert_eq!(path.get_attribute("fill"), Some("red"));
    assert_eq!(path.get_attribute("transform"), Some("scale(2)"));
}

#[test]
fn preserves_groups_with_animation_children() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![XastChild::Element(element(
                "g",
                &[],
                vec![XastChild::Element(element(
                    "animate",
                    &[("attributeName", "opacity")],
                    Vec::new(),
                ))],
            ))],
        ))],
    };

    collapse_groups::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(group) = &svg.children[0] else {
        panic!("expected group");
    };
    assert_eq!(group.name, "g");
}

#[test]
fn preserves_group_when_non_inheritable_attr_conflicts() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![XastChild::Element(element(
                "g",
                &[("opacity", ".5")],
                vec![XastChild::Element(element(
                    "path",
                    &[("opacity", "1"), ("d", "M0 0")],
                    Vec::new(),
                ))],
            ))],
        ))],
    };

    collapse_groups::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(group) = &svg.children[0] else {
        panic!("expected group");
    };
    assert_eq!(group.name, "g");
    assert_eq!(group.get_attribute("opacity"), Some(".5"));
}

#[test]
fn preserves_group_when_child_attr_is_animated() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![XastChild::Element(element(
                "g",
                &[("transform", "translate(10)")],
                vec![XastChild::Element(element(
                    "path",
                    &[("d", "M0 0")],
                    vec![XastChild::Element(element(
                        "animateTransform",
                        &[("attributeName", "transform")],
                        Vec::new(),
                    ))],
                ))],
            ))],
        ))],
    };

    collapse_groups::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(group) = &svg.children[0] else {
        panic!("expected group");
    };
    assert_eq!(group.name, "g");
    assert_eq!(group.get_attribute("transform"), Some("translate(10)"));
}

#[test]
fn preserves_group_under_switch() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![XastChild::Element(element(
                "switch",
                &[],
                vec![XastChild::Element(element(
                    "g",
                    &[],
                    vec![XastChild::Element(element("path", &[("d", "M0 0")], Vec::new()))],
                ))],
            ))],
        ))],
    };

    collapse_groups::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(switch) = &svg.children[0] else {
        panic!("expected switch");
    };
    let XastChild::Element(group) = &switch.children[0] else {
        panic!("expected group");
    };
    assert_eq!(group.name, "g");
}
