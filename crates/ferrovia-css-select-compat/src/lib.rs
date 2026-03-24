use ferrovia_css_what_compat::SelectorToken;

pub const fn select_all<'a, T>(_selector: &[SelectorToken], _root: &'a T) -> Vec<&'a T> {
    Vec::new()
}

pub const fn select_one<'a, T>(_selector: &[SelectorToken], _root: &'a T) -> Option<&'a T> {
    None
}

pub const fn is_match<T>(_selector: &[SelectorToken], _node: &T) -> bool {
    false
}
