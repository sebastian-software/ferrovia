use ferrovia_core::plugins::remove_xmlns;
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
fn removes_xmlns_from_svg_elements() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[
                ("xmlns", "http://www.w3.org/2000/svg"),
                ("viewBox", "0 0 10 10"),
            ],
            Vec::new(),
        ))],
    };

    remove_xmlns::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(svg.get_attribute("xmlns"), None);
    assert_eq!(svg.get_attribute("viewBox"), Some("0 0 10 10"));
}
