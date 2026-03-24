use ferrovia_core::plugins::remove_off_canvas_paths;
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
fn removes_path_outside_root_viewbox() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[("viewBox", "0 0 10 10")],
            vec![
                XastChild::Element(element("path", &[("d", "M20 20L30 30")], Vec::new())),
                XastChild::Element(element("path", &[("d", "M1 1L2 2")], Vec::new())),
            ],
        ))],
    };

    remove_off_canvas_paths::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(svg.children.len(), 1);
    let XastChild::Element(path) = &svg.children[0] else {
        panic!("expected path");
    };
    assert_eq!(path.get_attribute("d"), Some("M1 1L2 2"));
}

#[test]
fn preserves_transformed_subtree() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[("viewBox", "0 0 10 10")],
            vec![XastChild::Element(element(
                "g",
                &[("transform", "translate(100 100)")],
                vec![XastChild::Element(element(
                    "path",
                    &[("d", "M20 20L30 30")],
                    Vec::new(),
                ))],
            ))],
        ))],
    };

    remove_off_canvas_paths::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(svg.children.len(), 1);
}

#[test]
fn uses_width_height_when_viewbox_is_missing() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[("width", "10"), ("height", "10")],
            vec![XastChild::Element(element(
                "path",
                &[("d", "M20 20L30 30")],
                Vec::new(),
            ))],
        ))],
    };

    remove_off_canvas_paths::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert!(svg.children.is_empty());
}
