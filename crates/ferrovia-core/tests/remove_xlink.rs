use ferrovia_core::plugins::remove_xlink;
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
fn converts_xlink_href_and_removes_namespace() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[("xmlns:xlink", "http://www.w3.org/1999/xlink")],
            vec![XastChild::Element(element(
                "image",
                &[("xlink:href", "#shape")],
                Vec::new(),
            ))],
        ))],
    };

    remove_xlink::apply(&mut root, None).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(svg.get_attribute("xmlns:xlink"), None);
    let XastChild::Element(image) = &svg.children[0] else {
        panic!("expected image");
    };
    assert_eq!(image.get_attribute("href"), Some("#shape"));
    assert_eq!(image.get_attribute("xlink:href"), None);
}

#[test]
fn converts_xlink_title_and_show() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[("xmlns:xlink", "http://www.w3.org/1999/xlink")],
            vec![XastChild::Element(element(
                "a",
                &[("xlink:title", "Docs"), ("xlink:show", "new")],
                Vec::new(),
            ))],
        ))],
    };

    remove_xlink::apply(&mut root, None).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(link) = &svg.children[0] else {
        panic!("expected link");
    };
    assert_eq!(link.get_attribute("target"), Some("_blank"));
    let XastChild::Element(title) = &link.children[0] else {
        panic!("expected title child");
    };
    assert_eq!(title.name, "title");
    let XastChild::Text(XastText { value }) = &title.children[0] else {
        panic!("expected text");
    };
    assert_eq!(value, "Docs");
}

#[test]
fn preserves_namespace_for_legacy_elements_by_default() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[("xmlns:xlink", "http://www.w3.org/1999/xlink")],
            vec![XastChild::Element(element(
                "tref",
                &[("xlink:href", "#text")],
                Vec::new(),
            ))],
        ))],
    };

    remove_xlink::apply(&mut root, None).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(
        svg.get_attribute("xmlns:xlink"),
        Some("http://www.w3.org/1999/xlink")
    );
    let XastChild::Element(tref) = &svg.children[0] else {
        panic!("expected tref");
    };
    assert_eq!(tref.get_attribute("xlink:href"), Some("#text"));
    assert_eq!(tref.get_attribute("href"), None);
}

#[test]
fn converts_legacy_href_when_include_legacy_is_enabled() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[("xmlns:xlink", "http://www.w3.org/1999/xlink")],
            vec![XastChild::Element(element(
                "tref",
                &[("xlink:href", "#text")],
                Vec::new(),
            ))],
        ))],
    };
    let params = serde_json::json!({ "includeLegacy": true });

    remove_xlink::apply(&mut root, Some(&params)).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(svg.get_attribute("xmlns:xlink"), None);
    let XastChild::Element(tref) = &svg.children[0] else {
        panic!("expected tref");
    };
    assert_eq!(tref.get_attribute("xlink:href"), None);
    assert_eq!(tref.get_attribute("href"), Some("#text"));
}
