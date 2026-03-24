use ferrovia_core::plugins::remove_raster_images;
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
fn removes_image_elements_with_raster_href() {
    let mut root = XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[],
            vec![
                XastChild::Element(element("image", &[("xlink:href", "foo.png")], Vec::new())),
                XastChild::Element(element("image", &[("xlink:href", "#vector")], Vec::new())),
            ],
        ))],
    };

    remove_raster_images::apply(&mut root).expect("apply plugin");

    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    assert_eq!(svg.children.len(), 1);
    let XastChild::Element(image) = &svg.children[0] else {
        panic!("expected image");
    };
    assert_eq!(image.get_attribute("xlink:href"), Some("#vector"));
}
