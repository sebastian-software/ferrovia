use ferrovia_core::plugins::convert_colors;
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
fn converts_named_and_hex_colors_with_default_flow() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![XastChild::Element(element(
                "path",
                &[("fill", "navy"), ("stroke", "#ff0000")],
                Vec::new(),
            ))],
        ))],
    };

    convert_colors::apply(&mut root, None).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(path) = &svg.children[0] else {
        panic!("expected path");
    };
    assert_eq!(path.get_attribute("fill"), Some("navy"));
    assert_eq!(path.get_attribute("stroke"), Some("red"));
}

#[test]
fn converts_rgb_function_to_hex() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![XastChild::Element(element(
                "path",
                &[("fill", "rgb(50%, 100, 100%)")],
                Vec::new(),
            ))],
        ))],
    };

    let params = serde_json::json!({
        "shorthex": false,
        "shortname": false,
        "convertCase": "lower"
    });
    convert_colors::apply(&mut root, Some(&params)).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(path) = &svg.children[0] else {
        panic!("expected path");
    };
    assert_eq!(path.get_attribute("fill"), Some("#8064ff"));
}

#[test]
fn converts_current_color_with_bool_and_respects_mask_scope() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![
                XastChild::Element(element("path", &[("fill", "red")], Vec::new())),
                XastChild::Element(element(
                    "mask",
                    &[],
                    vec![XastChild::Element(element(
                        "path",
                        &[("fill", "red")],
                        Vec::new(),
                    ))],
                )),
            ],
        ))],
    };

    let params = serde_json::json!({ "currentColor": true });
    convert_colors::apply(&mut root, Some(&params)).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(path) = &svg.children[0] else {
        panic!("expected path");
    };
    assert_eq!(path.get_attribute("fill"), Some("currentColor"));

    let XastChild::Element(mask) = &svg.children[1] else {
        panic!("expected mask");
    };
    let XastChild::Element(mask_path) = &mask.children[0] else {
        panic!("expected mask path");
    };
    assert_eq!(mask_path.get_attribute("fill"), Some("red"));
}

#[test]
fn converts_current_color_with_exact_string_and_regex_literal() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![
                XastChild::Element(element("path", &[("fill", "#00F")], Vec::new())),
                XastChild::Element(element(
                    "path",
                    &[("stroke", "rgb(1, 2, 3)")],
                    Vec::new(),
                )),
            ],
        ))],
    };

    let params = serde_json::json!({
        "currentColor": "#00F",
        "rgb2hex": false,
        "names2hex": false,
        "shorthex": false,
        "shortname": false,
        "convertCase": false
    });
    convert_colors::apply(&mut root, Some(&params)).expect("apply plugin");

    let regex_params = serde_json::json!({
        "currentColor": "/^rgb\\(/i",
        "rgb2hex": false,
        "names2hex": false,
        "shorthex": false,
        "shortname": false,
        "convertCase": false
    });
    convert_colors::apply(&mut root, Some(&regex_params)).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(fill_path) = &svg.children[0] else {
        panic!("expected fill path");
    };
    let XastChild::Element(stroke_path) = &svg.children[1] else {
        panic!("expected stroke path");
    };
    assert_eq!(fill_path.get_attribute("fill"), Some("currentColor"));
    assert_eq!(stroke_path.get_attribute("stroke"), Some("currentColor"));
}

#[test]
fn preserves_case_for_url_references() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![XastChild::Element(element(
                "path",
                &[("fill", "url(#PaintServer)")],
                Vec::new(),
            ))],
        ))],
    };

    convert_colors::apply(&mut root, None).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(path) = &svg.children[0] else {
        panic!("expected path");
    };
    assert_eq!(path.get_attribute("fill"), Some("url(#PaintServer)"));
}
