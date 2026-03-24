use ferrovia_core::plugins::remove_scripts;
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
fn removes_script_elements_and_event_attributes() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[("onclick", "alert(1)")],
            vec![
                XastChild::Element(element("script", &[], Vec::new())),
                XastChild::Element(element("g", &[("onload", "boot()")], Vec::new())),
            ],
        ))],
    };

    remove_scripts::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(svg.get_attribute("onclick"), None);
    assert_eq!(svg.children.len(), 1);
    let XastChild::Element(group) = &svg.children[0] else {
        panic!("expected group");
    };
    assert_eq!(group.get_attribute("onload"), None);
}

#[test]
fn unwraps_anchor_with_javascript_href() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![XastChild::Element(element(
                "a",
                &[("href", "javascript:alert(1)")],
                vec![
                    XastChild::Text(XastText {
                        value: " ".to_string(),
                    }),
                    XastChild::Element(element("g", &[], Vec::new())),
                ],
            ))],
        ))],
    };

    remove_scripts::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(svg.children.len(), 1);
    let XastChild::Element(group) = &svg.children[0] else {
        panic!("expected group");
    };
    assert_eq!(group.name, "g");
}
