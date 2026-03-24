use ferrovia_core::plugins::remove_unused_ns;
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
fn removes_unused_namespace_declarations_from_root_svg() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[
                ("xmlns", "http://www.w3.org/2000/svg"),
                ("xmlns:xlink", "http://www.w3.org/1999/xlink"),
                (
                    "xmlns:inkscape",
                    "http://www.inkscape.org/namespaces/inkscape",
                ),
            ],
            vec![XastChild::Element(element("g", &[], Vec::new()))],
        ))],
    };

    remove_unused_ns::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(svg.get_attribute("xmlns:xlink"), None);
    assert_eq!(svg.get_attribute("xmlns:inkscape"), None);
    assert_eq!(
        svg.get_attribute("xmlns"),
        Some("http://www.w3.org/2000/svg")
    );
}

#[test]
fn preserves_namespaces_used_in_element_or_attribute_names() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[
                ("xmlns:xlink", "http://www.w3.org/1999/xlink"),
                (
                    "xmlns:sodipodi",
                    "http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd",
                ),
            ],
            vec![
                XastChild::Element(element("sodipodi:namedview", &[], Vec::new())),
                XastChild::Element(element("image", &[("xlink:href", "#id")], Vec::new())),
            ],
        ))],
    };

    remove_unused_ns::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(
        svg.get_attribute("xmlns:xlink"),
        Some("http://www.w3.org/1999/xlink")
    );
    assert_eq!(
        svg.get_attribute("xmlns:sodipodi"),
        Some("http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd")
    );
}
