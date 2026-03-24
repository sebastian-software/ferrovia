use ferrovia_core::plugins::remove_hidden_elems;
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
fn removes_hidden_circle_and_display_none_path() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[("xmlns", "http://www.w3.org/2000/svg")],
            vec![
                XastChild::Element(element("circle", &[("r", "0")], Vec::new())),
                XastChild::Element(element(
                    "path",
                    &[("d", "M0 0"), ("display", "none")],
                    Vec::new(),
                )),
            ],
        ))],
    };

    remove_hidden_elems::apply(&mut root, None).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert!(svg.children.is_empty());
}

#[test]
fn keeps_hidden_container_with_visible_descendant() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[("xmlns", "http://www.w3.org/2000/svg")],
            vec![XastChild::Element(element(
                "g",
                &[("visibility", "hidden")],
                vec![XastChild::Element(element(
                    "path",
                    &[("d", "M0 0"), ("visibility", "visible")],
                    Vec::new(),
                ))],
            ))],
        ))],
    };

    remove_hidden_elems::apply(&mut root, None).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(svg.children.len(), 1);
}

#[test]
fn keeps_marker_with_display_none() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[("xmlns", "http://www.w3.org/2000/svg")],
            vec![
                XastChild::Element(element(
                    "defs",
                    &[],
                    vec![XastChild::Element(element(
                        "marker",
                        &[("id", "m"), ("display", "none")],
                        Vec::new(),
                    ))],
                )),
                XastChild::Element(element("path", &[("d", "M0 0L10 10"), ("marker-start", "url(#m)")], Vec::new())),
            ],
        ))],
    };

    remove_hidden_elems::apply(&mut root, None).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(svg.children.len(), 2);
}

#[test]
fn removes_empty_and_single_point_paths_without_markers() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[("xmlns", "http://www.w3.org/2000/svg")],
            vec![
                XastChild::Element(element("path", &[], Vec::new())),
                XastChild::Element(element("path", &[("d", "M0 0")], Vec::new())),
            ],
        ))],
    };

    remove_hidden_elems::apply(&mut root, None).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert!(svg.children.is_empty());
}

#[test]
fn keeps_single_point_path_with_marker() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[("xmlns", "http://www.w3.org/2000/svg")],
            vec![XastChild::Element(element(
                "path",
                &[("d", "M0 0"), ("marker-start", "url(#m)")],
                Vec::new(),
            ))],
        ))],
    };

    remove_hidden_elems::apply(&mut root, None).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(svg.children.len(), 1);
}

#[test]
fn removes_unreferenced_non_rendering_defs_and_empty_defs() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[("xmlns", "http://www.w3.org/2000/svg")],
            vec![XastChild::Element(element(
                "defs",
                &[],
                vec![XastChild::Element(element(
                    "marker",
                    &[("id", "m")],
                    Vec::new(),
                ))],
            ))],
        ))],
    };

    remove_hidden_elems::apply(&mut root, None).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert!(svg.children.is_empty());
}

#[test]
fn keeps_non_rendering_defs_when_styles_deopt_cleanup() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[("xmlns", "http://www.w3.org/2000/svg")],
            vec![
                XastChild::Element(element(
                    "style",
                    &[],
                    vec![XastChild::Text(ferrovia_core::types::XastText {
                        value: ["path", "{fill:red}"].concat(),
                    })],
                )),
                XastChild::Element(element(
                    "defs",
                    &[],
                    vec![XastChild::Element(element(
                        "marker",
                        &[("id", "m")],
                        Vec::new(),
                    ))],
                )),
            ],
        ))],
    };

    remove_hidden_elems::apply(&mut root, None).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(svg.children.len(), 2);
}
