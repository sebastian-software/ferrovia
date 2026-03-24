use ferrovia_core::plugins::convert_shape_to_path;
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
fn converts_rect_and_line_to_path() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![
                XastChild::Element(element(
                    "rect",
                    &[("x", "1"), ("y", "2"), ("width", "3"), ("height", "4")],
                    Vec::new(),
                )),
                XastChild::Element(element(
                    "line",
                    &[("x1", "0"), ("y1", "1"), ("x2", "2"), ("y2", "3")],
                    Vec::new(),
                )),
            ],
        ))],
    };

    convert_shape_to_path::apply(&mut root, None).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(rect_path) = &svg.children[0] else {
        panic!("expected rect path");
    };
    assert_eq!(rect_path.name, "path");
    assert_eq!(rect_path.get_attribute("d"), Some("M1 2H4V6H1z"));

    let XastChild::Element(line_path) = &svg.children[1] else {
        panic!("expected line path");
    };
    assert_eq!(line_path.name, "path");
    assert_eq!(line_path.get_attribute("d"), Some("M0 1L2 3"));
}

#[test]
fn converts_poly_shapes_and_removes_too_short_polyline() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![
                XastChild::Element(element(
                    "polyline",
                    &[("points", "1 2 3 4 5 6")],
                    Vec::new(),
                )),
                XastChild::Element(element("polygon", &[("points", "1,2 3,4 5,6")], Vec::new())),
                XastChild::Element(element("polyline", &[("points", "1 2")], Vec::new())),
            ],
        ))],
    };

    convert_shape_to_path::apply(&mut root, None).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(svg.children.len(), 2);
    let XastChild::Element(polyline_path) = &svg.children[0] else {
        panic!("expected polyline path");
    };
    assert_eq!(polyline_path.get_attribute("d"), Some("M1 2L3 4L5 6"));
    let XastChild::Element(polygon_path) = &svg.children[1] else {
        panic!("expected polygon path");
    };
    assert_eq!(polygon_path.get_attribute("d"), Some("M1 2L3 4L5 6z"));
}

#[test]
fn optionally_converts_circle_and_ellipse_to_arc_paths() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![
                XastChild::Element(element(
                    "circle",
                    &[("cx", "10"), ("cy", "20"), ("r", "5")],
                    Vec::new(),
                )),
                XastChild::Element(element(
                    "ellipse",
                    &[("cx", "10"), ("cy", "20"), ("rx", "5"), ("ry", "3")],
                    Vec::new(),
                )),
            ],
        ))],
    };
    let params = serde_json::json!({ "convertArcs": true, "floatPrecision": 2 });

    convert_shape_to_path::apply(&mut root, Some(&params)).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(circle) = &svg.children[0] else {
        panic!("expected circle path");
    };
    assert_eq!(circle.name, "path");
    assert_eq!(
        circle.get_attribute("d"),
        Some("M10 15A5 5 0 1 0 10 25A5 5 0 1 0 10 15z")
    );

    let XastChild::Element(ellipse) = &svg.children[1] else {
        panic!("expected ellipse path");
    };
    assert_eq!(ellipse.name, "path");
    assert_eq!(
        ellipse.get_attribute("d"),
        Some("M10 17A5 3 0 1 0 10 23A5 3 0 1 0 10 17z")
    );
}
