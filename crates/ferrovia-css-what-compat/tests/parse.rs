use ferrovia_css_what_compat::{Combinator, parse};

#[test]
fn parses_basic_selector_groups() {
    let groups = parse("svg > g.hero path[fill=red], #main");
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].tokens.len(), 3);
    assert_eq!(groups[0].tokens[1].combinator, Some(Combinator::Child));
    assert_eq!(groups[0].tokens[2].combinator, Some(Combinator::Descendant));
    assert_eq!(groups[0].tokens[1].compound.classes, vec!["hero"]);
    assert_eq!(groups[0].tokens[2].compound.attributes[0].name, "fill");
    assert_eq!(groups[1].tokens[0].compound.id.as_deref(), Some("main"));
}
