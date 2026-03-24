use ferrovia_core::plugins::cleanup_enable_background;
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
fn removes_enable_background_when_no_filter_exists() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[
                ("width", "100"),
                ("height", "50"),
                ("enable-background", "new 0 0 100 50"),
                ("style", "fill:red;enable-background:new 0 0 100 50"),
            ],
            Vec::new(),
        ))],
    };

    cleanup_enable_background::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(svg.get_attribute("enable-background"), None);
    assert_eq!(svg.get_attribute("style"), Some("fill:red"));
}

#[test]
fn collapses_matching_svg_enable_background_with_filter_present() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[
                ("width", "100"),
                ("height", "50"),
                ("enable-background", "new 0 0 100 50"),
                ("style", "enable-background:new 0 0 100 50;fill:red"),
            ],
            vec![XastChild::Element(element("filter", &[], Vec::new()))],
        ))],
    };

    cleanup_enable_background::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(svg.get_attribute("enable-background"), None);
    assert_eq!(svg.get_attribute("style"), Some("fill:red"));
}

#[test]
fn collapses_matching_mask_to_new() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![
                XastChild::Element(element("filter", &[], Vec::new())),
                XastChild::Element(element(
                    "mask",
                    &[
                        ("width", "20"),
                        ("height", "10"),
                        ("enable-background", "new 0 0 20 10"),
                    ],
                    Vec::new(),
                )),
            ],
        ))],
    };

    cleanup_enable_background::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(mask) = &svg.children[1] else {
        panic!("expected mask");
    };
    assert_eq!(mask.get_attribute("enable-background"), Some("new"));
}
