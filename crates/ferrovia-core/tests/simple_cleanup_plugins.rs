use ferrovia_core::plugins::{
    remove_desc, remove_dimensions, remove_editors_ns_data, remove_empty_attrs, remove_empty_text,
};
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
fn remove_desc_drops_standard_editor_content() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![
                XastChild::Element(element("desc", &[], vec![text("Created with Inkscape")])),
                XastChild::Element(element("desc", &[], vec![text("Accessible description")])),
            ],
        ))],
    };

    remove_desc::apply(&mut root, None).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(svg.children.len(), 1);
    let XastChild::Element(desc) = &svg.children[0] else {
        panic!("expected desc");
    };
    assert_eq!(desc.name, "desc");
}

#[test]
fn remove_dimensions_creates_viewbox_from_numeric_size() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[("width", "100"), ("height", "50")],
            Vec::new(),
        ))],
    };

    remove_dimensions::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(svg.get_attribute("width"), None);
    assert_eq!(svg.get_attribute("height"), None);
    assert_eq!(svg.get_attribute("viewBox"), Some("0 0 100 50"));
}

#[test]
fn remove_editors_ns_data_strips_editor_prefixes() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[(
                "xmlns:sodipodi",
                "http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd",
            )],
            vec![
                XastChild::Element(element("sodipodi:namedview", &[], Vec::new())),
                XastChild::Element(element(
                    "path",
                    &[("sodipodi:nodetypes", "cccc"), ("d", "M0 0")],
                    Vec::new(),
                )),
            ],
        ))],
    };

    remove_editors_ns_data::apply(&mut root, None).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(svg.get_attribute("xmlns:sodipodi"), None);
    assert_eq!(svg.children.len(), 1);
    let XastChild::Element(path) = &svg.children[0] else {
        panic!("expected path");
    };
    assert_eq!(path.get_attribute("sodipodi:nodetypes"), None);
    assert_eq!(path.get_attribute("d"), Some("M0 0"));
}

#[test]
fn remove_empty_attrs_preserves_conditional_processing_attrs() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![XastChild::Element(element(
                "path",
                &[("fill", ""), ("requiredFeatures", "")],
                Vec::new(),
            ))],
        ))],
    };

    remove_empty_attrs::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(path) = &svg.children[0] else {
        panic!("expected path");
    };
    assert_eq!(path.get_attribute("fill"), None);
    assert_eq!(path.get_attribute("requiredFeatures"), Some(""));
}

#[test]
fn remove_empty_text_removes_empty_text_like_nodes() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![
                XastChild::Element(element("text", &[], Vec::new())),
                XastChild::Element(element("tspan", &[], Vec::new())),
                XastChild::Element(element("tref", &[], Vec::new())),
                XastChild::Element(element("tref", &[("xlink:href", "#ref")], Vec::new())),
            ],
        ))],
    };

    remove_empty_text::apply(&mut root, None).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(svg.children.len(), 1);
    let XastChild::Element(tref) = &svg.children[0] else {
        panic!("expected tref");
    };
    assert_eq!(tref.get_attribute("xlink:href"), Some("#ref"));
}
