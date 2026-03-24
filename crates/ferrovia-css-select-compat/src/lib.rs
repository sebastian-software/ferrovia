use ferrovia_css_what_compat::{Combinator, CompoundSelector, SelectorGroup};

pub trait Adapter<'a, Node> {
    fn is_tag(&self, node: &Node) -> bool;
    fn children(&self, node: &'a Node) -> &'a [Node];
    fn matches_compound(&self, node: &Node, compound: &CompoundSelector) -> bool;
}

#[must_use]
pub fn select_all<'a, Node, A>(
    selectors: &[SelectorGroup],
    roots: &'a [Node],
    adapter: &A,
) -> Vec<&'a Node>
where
    A: Adapter<'a, Node>,
{
    let mut matches = Vec::<&'a Node>::new();
    let mut ancestry = Vec::<&'a Node>::new();
    collect_matches(selectors, roots, adapter, &mut ancestry, &mut matches);
    matches
}

#[must_use]
pub fn select_one<'a, Node, A>(
    selectors: &[SelectorGroup],
    roots: &'a [Node],
    adapter: &A,
) -> Option<&'a Node>
where
    A: Adapter<'a, Node>,
{
    select_all(selectors, roots, adapter).into_iter().next()
}

#[must_use]
pub fn is_match<'a, Node, A>(
    selectors: &[SelectorGroup],
    node: &'a Node,
    ancestry: &[&'a Node],
    adapter: &A,
) -> bool
where
    A: Adapter<'a, Node>,
{
    selectors
        .iter()
        .any(|selector| selector_matches(selector, node, ancestry, adapter))
}

fn collect_matches<'a, Node, A>(
    selectors: &[SelectorGroup],
    nodes: &'a [Node],
    adapter: &A,
    ancestry: &mut Vec<&'a Node>,
    matches: &mut Vec<&'a Node>,
) where
    A: Adapter<'a, Node>,
{
    for node in nodes {
        if !adapter.is_tag(node) {
            continue;
        }
        if is_match(selectors, node, ancestry.as_slice(), adapter) {
            matches.push(node);
        }
        ancestry.push(node);
        collect_matches(
            selectors,
            adapter.children(node),
            adapter,
            ancestry,
            matches,
        );
        ancestry.pop();
    }
}

fn selector_matches<'a, Node, A>(
    selector: &SelectorGroup,
    node: &'a Node,
    ancestry: &[&'a Node],
    adapter: &A,
) -> bool
where
    A: Adapter<'a, Node>,
{
    if selector.tokens.is_empty() {
        return false;
    }
    let last_index = selector.tokens.len() - 1;
    if !adapter.matches_compound(node, &selector.tokens[last_index].compound) {
        return false;
    }
    if last_index == 0 {
        return true;
    }
    match_selector_ancestors(selector, last_index - 1, ancestry, adapter)
}

fn match_selector_ancestors<'a, Node, A>(
    selector: &SelectorGroup,
    token_index: usize,
    ancestry: &[&'a Node],
    adapter: &A,
) -> bool
where
    A: Adapter<'a, Node>,
{
    let token = &selector.tokens[token_index];
    let combinator = selector.tokens[token_index + 1]
        .combinator
        .unwrap_or(Combinator::Descendant);

    match combinator {
        Combinator::Child => ancestry.last().is_some_and(|parent| {
            adapter.matches_compound(parent, &token.compound)
                && (token_index == 0
                    || match_selector_ancestors(
                        selector,
                        token_index - 1,
                        &ancestry[..ancestry.len() - 1],
                        adapter,
                    ))
        }),
        Combinator::Descendant => {
            let mut index = ancestry.len();
            while index > 0 {
                index -= 1;
                if adapter.matches_compound(ancestry[index], &token.compound)
                    && (token_index == 0
                        || match_selector_ancestors(
                            selector,
                            token_index - 1,
                            &ancestry[..index],
                            adapter,
                        ))
                {
                    return true;
                }
            }
            false
        }
    }
}
