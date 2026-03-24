use ferrovia_core::path::{parse_path_data, stringify_path_data};
use ferrovia_core::style::{collect_stylesheet, parse_style_declarations};
use ferrovia_core::svgo::tools::{cleanup_out_data, find_references, includes_url_reference};
use ferrovia_core::types::{
    XastChild, XastElement, XastRoot, XastText,
};

#[test]
fn parses_basic_path_data_items() {
    let path = parse_path_data("M0 0L10 10z");
    assert_eq!(path.len(), 3);
    assert_eq!(path[0].command, 'M');
    assert_eq!(path[1].command, 'L');
    assert_eq!(path[2].command, 'z');
}

#[test]
fn stringifies_basic_path_data_items() {
    let path = parse_path_data("M0 0L10 10z");
    assert_eq!(stringify_path_data(&path, Some(1)), "M0 0L10 10z");
}

#[test]
fn parses_basic_style_declarations() {
    let declarations = parse_style_declarations("fill:red;stroke:blue");
    assert_eq!(declarations.len(), 2);
    assert_eq!(declarations[0].name, "fill");
    assert_eq!(declarations[1].value, "blue");
}

#[test]
fn collects_stylesheet_rules_from_style_nodes() {
    let root = XastRoot {
        children: vec![XastChild::Element(XastElement {
            name: "svg".to_string(),
            attributes: Vec::new(),
            children: vec![XastChild::Element(XastElement {
                name: "style".to_string(),
                attributes: Vec::new(),
                children: vec![XastChild::Text(XastText {
                    value: ".hero{fill:red;unknown:keep}".to_string(),
                })],
            })],
        })],
    };

    let stylesheet = collect_stylesheet(&root);
    assert_eq!(stylesheet.rules.len(), 1);
    assert_eq!(stylesheet.rules[0].selector, ".hero");
    assert_eq!(stylesheet.rules[0].declarations.len(), 1);
}

#[test]
fn finds_svg_references_like_svgo_tools_layer() {
    assert!(includes_url_reference("fill:url(#paint)"));
    assert_eq!(find_references("fill", "url(#paint)"), vec!["paint"]);
    assert_eq!(find_references("href", "#shape"), vec!["shape"]);
    assert_eq!(find_references("begin", "target.begin+1s"), vec!["target"]);
    assert_eq!(cleanup_out_data(&[0.0, -1.0, 0.5, 0.5], false), "0-1 .5.5");
}
