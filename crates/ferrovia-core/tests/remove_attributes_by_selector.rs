use ferrovia_core::plugins::remove_attributes_by_selector;
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
fn removes_single_attribute_for_matching_selector() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![XastChild::Element(element(
                "path",
                &[("id", "main"), ("fill", "#00ff00"), ("stroke", "#00ff00")],
                Vec::new(),
            ))],
        ))],
    };

    let params = serde_json::json!({
        "selector": "[fill='#00ff00']",
        "attributes": "fill"
    });

    remove_attributes_by_selector::apply(&mut root, Some(&params)).expect("apply plugin");
    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(path) = &svg.children[0] else {
        panic!("expected path");
    };
    assert_eq!(path.get_attribute("fill"), None);
    assert_eq!(path.get_attribute("stroke"), Some("#00ff00"));
}

#[test]
fn removes_multiple_attributes_for_multiple_selectors() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![
                XastChild::Element(element(
                    "path",
                    &[("id", "main"), ("fill", "#00ff00"), ("stroke", "#00ff00")],
                    Vec::new(),
                )),
                XastChild::Element(element(
                    "rect",
                    &[("id", "remove"), ("stroke", "#00ff00"), ("fill", "#00ff00")],
                    Vec::new(),
                )),
            ],
        ))],
    };

    let params = serde_json::json!({
        "selectors": [
            {
                "selector": "[fill='#00ff00']",
                "attributes": "fill"
            },
            {
                "selector": "#remove",
                "attributes": ["stroke", "id"]
            }
        ]
    });

    remove_attributes_by_selector::apply(&mut root, Some(&params)).expect("apply plugin");
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
    assert_eq!(rect.get_attribute("fill"), None);
    assert_eq!(rect.get_attribute("stroke"), None);
    assert_eq!(rect.get_attribute("id"), None);
}
