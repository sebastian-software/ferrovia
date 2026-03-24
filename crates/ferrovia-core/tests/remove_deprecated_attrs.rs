use ferrovia_core::plugins::remove_deprecated_attrs;
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
fn removes_safe_svg_deprecated_attrs_by_default() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[("version", "1.1"), ("baseProfile", "tiny")],
            Vec::new(),
        ))],
    };

    remove_deprecated_attrs::apply(&mut root, None).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(svg.get_attribute("version"), None);
    assert_eq!(svg.get_attribute("baseProfile"), Some("tiny"));
}

#[test]
fn removes_unsafe_attrs_when_enabled() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "g",
            &[("enable-background", "new"), ("clip", "rect(0 0 0 0)")],
            Vec::new(),
        ))],
    };
    let params = serde_json::json!({ "removeUnsafe": true });

    remove_deprecated_attrs::apply(&mut root, Some(&params)).expect("apply plugin");

    let XastChild::Element(group) = &root.children[0] else {
        panic!("expected group");
    };
    assert_eq!(group.get_attribute("enable-background"), None);
    assert_eq!(group.get_attribute("clip"), None);
}

#[test]
fn removes_xml_lang_when_lang_exists_and_selector_does_not_use_it() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![
                XastChild::Element(element(
                    "style",
                    &[],
                    vec![XastChild::Text(XastText {
                        value: ["text[lang=en]", "{fill:red}"].concat(),
                    })],
                )),
                XastChild::Element(element(
                    "text",
                    &[("xml:lang", "en"), ("lang", "en")],
                    Vec::new(),
                )),
            ],
        ))],
    };

    remove_deprecated_attrs::apply(&mut root, None).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(text) = &svg.children[1] else {
        panic!("expected text");
    };
    assert_eq!(text.get_attribute("xml:lang"), None);
    assert_eq!(text.get_attribute("lang"), Some("en"));
}

#[test]
fn preserves_xml_lang_when_stylesheet_selector_uses_it() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![
                XastChild::Element(element(
                    "style",
                    &[],
                    vec![XastChild::Text(XastText {
                        value: ["text[xml:lang=en]", "{fill:red}"].concat(),
                    })],
                )),
                XastChild::Element(element(
                    "text",
                    &[("xml:lang", "en"), ("lang", "en")],
                    Vec::new(),
                )),
            ],
        ))],
    };

    remove_deprecated_attrs::apply(&mut root, None).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(text) = &svg.children[1] else {
        panic!("expected text");
    };
    assert_eq!(text.get_attribute("xml:lang"), Some("en"));
}
