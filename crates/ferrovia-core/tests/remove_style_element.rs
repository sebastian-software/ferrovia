use ferrovia_core::plugins::remove_style_element;
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

#[test]
fn removes_style_elements_recursively() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![
                XastChild::Element(element(
                    "style",
                    &[],
                    vec![XastChild::Text(XastText {
                        value: ".a{fill:red}".to_string(),
                    })],
                )),
                XastChild::Element(element(
                    "g",
                    &[],
                    vec![XastChild::Element(element("style", &[], Vec::new()))],
                )),
            ],
        ))],
    };

    remove_style_element::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert!(
        svg.children.iter().all(|child| {
            matches!(child, XastChild::Element(element) if element.name != "style")
        })
    );
}
