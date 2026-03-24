use ferrovia_core::svgo::css_select_adapter::CssSelectAdapter;
use ferrovia_core::types::{XastAttribute, XastChild, XastElement, XastRoot, XastText};
use ferrovia_core::xast::{detach_node_from_parent, matches, query_selector, query_selector_all};

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

fn query_fixture() -> XastRoot {
    XastRoot {
        children: vec![XastChild::Element(element(
            "svg",
            &[("id", "root")],
            vec![
                XastChild::Element(element(
                    "defs",
                    &[],
                    vec![XastChild::Element(element(
                        "linearGradient",
                        &[("id", "paint")],
                        Vec::new(),
                    ))],
                )),
                XastChild::Element(element(
                    "g",
                    &[("class", "hero group")],
                    vec![
                        XastChild::Element(element(
                            "path",
                            &[("id", "main"), ("fill", "red"), ("class", "hero-path")],
                            Vec::new(),
                        )),
                        XastChild::Element(element(
                            "text",
                            &[("data-role", "label")],
                            vec![XastChild::Text(XastText {
                                value: "Hello".to_string(),
                            })],
                        )),
                    ],
                )),
            ],
        ))],
    }
}

#[test]
fn query_selector_all_supports_basic_selector_shapes() {
    let root = query_fixture();
    assert_eq!(query_selector_all(&root, "path").len(), 1);
    assert_eq!(query_selector_all(&root, "#main").len(), 1);
    assert_eq!(query_selector_all(&root, ".hero").len(), 1);
    assert_eq!(query_selector_all(&root, "[fill=red]").len(), 1);
    assert_eq!(query_selector_all(&root, "[data-role]").len(), 1);
}

#[test]
fn query_selector_supports_descendant_and_child_combinators() {
    let root = query_fixture();
    assert!(query_selector(&root, "g path").is_some());
    assert!(query_selector(&root, "svg > g").is_some());
    assert!(query_selector(&root, "defs > linearGradient").is_some());
    assert!(query_selector(&root, "svg > path").is_none());
}

#[test]
fn matches_checks_compound_selector_against_element() {
    let root = query_fixture();
    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let XastChild::Element(group) = &svg.children[1] else {
        panic!("expected group");
    };
    let XastChild::Element(path) = &group.children[0] else {
        panic!("expected path");
    };

    assert!(matches(path, "path#main.hero-path[fill=red]"));
    assert!(!matches(path, "g path"));
}

#[test]
fn detach_node_from_parent_removes_requested_child() {
    let mut root = query_fixture();
    let XastChild::Element(svg) = &mut root.children[0] else {
        panic!("expected svg");
    };
    detach_node_from_parent(&mut svg.children, 0);
    assert_eq!(svg.children.len(), 1);
    let XastChild::Element(group) = &svg.children[0] else {
        panic!("expected group");
    };
    assert_eq!(group.name, "g");
}

#[test]
fn css_select_adapter_exposes_xast_navigation() {
    let root = query_fixture();
    let adapter = CssSelectAdapter::new(&root);
    let XastChild::Element(svg) = &root.children[0] else {
        panic!("expected svg");
    };
    let group_child = &svg.children[1];
    let XastChild::Element(group) = group_child else {
        panic!("expected group");
    };
    let path_child = &group.children[0];
    let XastChild::Element(path) = path_child else {
        panic!("expected path");
    };

    assert!(CssSelectAdapter::is_tag(group_child));
    assert_eq!(CssSelectAdapter::get_children(group_child).len(), 2);
    assert_eq!(
        CssSelectAdapter::get_attribute_value(path, "fill"),
        Some("red")
    );
    assert!(CssSelectAdapter::has_attrib(path, "id"));
    assert_eq!(CssSelectAdapter::get_text(group), "");
    assert!(adapter.get_parent(path_child).is_some());
    assert_eq!(adapter.get_siblings(path_child).len(), 2);
}
