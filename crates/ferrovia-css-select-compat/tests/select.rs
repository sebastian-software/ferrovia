use ferrovia_css_select_compat::{Adapter, is_match, select_all, select_one};
use ferrovia_css_what_compat::{CompoundSelector, parse};

#[derive(Debug)]
struct Node {
    name: &'static str,
    id: Option<&'static str>,
    class: Option<&'static str>,
    children: Vec<Self>,
}

struct NodeAdapter;

impl Adapter<'_, Node> for NodeAdapter {
    fn is_tag(&self, _node: &Node) -> bool {
        true
    }

    fn children<'a>(&self, node: &'a Node) -> &'a [Node] {
        node.children.as_slice()
    }

    fn matches_compound(&self, node: &Node, compound: &CompoundSelector) -> bool {
        if !compound.universal
            && let Some(tag) = &compound.tag
            && node.name != tag
        {
            return false;
        }
        if let Some(id) = &compound.id
            && node.id != Some(id.as_str())
        {
            return false;
        }
        for class_name in &compound.classes {
            let Some(classes) = node.class else {
                return false;
            };
            if !classes
                .split_ascii_whitespace()
                .any(|class| class == class_name)
            {
                return false;
            }
        }
        true
    }
}

fn fixture() -> Vec<Node> {
    vec![Node {
        name: "svg",
        id: Some("root"),
        class: None,
        children: vec![Node {
            name: "g",
            id: None,
            class: Some("hero"),
            children: vec![Node {
                name: "path",
                id: Some("main"),
                class: Some("hero-path"),
                children: Vec::new(),
            }],
        }],
    }]
}

#[test]
fn selects_basic_nodes() {
    let root = fixture();
    let selectors = parse("svg > g path");
    let adapter = NodeAdapter;
    let all = select_all(&selectors, root.as_slice(), &adapter);
    assert_eq!(all.len(), 1);
    let one = select_one(&selectors, root.as_slice(), &adapter);
    assert!(one.is_some());
    let ancestry = vec![&root[0], &root[0].children[0]];
    assert!(is_match(
        &parse("path#main.hero-path"),
        &root[0].children[0].children[0],
        &ancestry,
        &adapter
    ));
}
