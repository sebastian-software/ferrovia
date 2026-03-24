use ferrovia_core::plugins::remove_empty_containers;
use ferrovia_core::types::{XastAttribute, XastChild, XastElement, XastRoot, XastText};

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

fn text(value: &str) -> XastChild {
    XastChild::Text(XastText {
        value: value.to_string(),
    })
}

#[test]
fn removes_empty_defs_and_uses_that_reference_them() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![
                XastChild::Element(element("defs", &[("id", "gone")], Vec::new())),
                XastChild::Element(element("use", &[("xlink:href", "#gone")], Vec::new())),
                XastChild::Element(element("g", &[], vec![text("keep")])),
            ],
        ))],
    };

    remove_empty_containers::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(svg.children.len(), 1);
    let XastChild::Element(group) = &svg.children[0] else {
        panic!("expected group");
    };
    assert_eq!(group.name, "g");
}

#[test]
fn preserves_empty_pattern_with_attributes_and_filtered_group() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![
                XastChild::Element(element("pattern", &[("width", "10")], Vec::new())),
                XastChild::Element(element("g", &[("filter", "url(#f)")], Vec::new())),
            ],
        ))],
    };

    remove_empty_containers::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(svg.children.len(), 2);
}
