#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectorToken {
    pub value: String,
}

#[must_use]
pub fn parse(input: &str) -> Vec<SelectorToken> {
    vec![SelectorToken {
        value: input.to_string(),
    }]
}
