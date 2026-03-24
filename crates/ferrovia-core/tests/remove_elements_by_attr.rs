use ferrovia_core::plugins::remove_elements_by_attr;
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
fn removes_elements_by_id() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![
                XastChild::Element(element("path", &[("id", "remove-me")], Vec::new())),
                XastChild::Element(element("rect", &[("id", "keep-me")], Vec::new())),
            ],
        ))],
    };

    let params = serde_json::json!({ "id": "remove-me" });
    remove_elements_by_attr::apply(&mut root, Some(&params)).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(svg.children.len(), 1);
    let XastChild::Element(rect) = &svg.children[0] else {
        panic!("expected rect");
    };
    assert_eq!(rect.name, "rect");
}

#[test]
fn removes_elements_by_class_membership() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![
                XastChild::Element(element("path", &[("class", "hero remove")], Vec::new())),
                XastChild::Element(element("rect", &[("class", "hero keep")], Vec::new())),
            ],
        ))],
    };

    let params = serde_json::json!({ "class": ["remove"] });
    remove_elements_by_attr::apply(&mut root, Some(&params)).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(svg.children.len(), 1);
    let XastChild::Element(rect) = &svg.children[0] else {
        panic!("expected rect");
    };
    assert_eq!(rect.name, "rect");
}
