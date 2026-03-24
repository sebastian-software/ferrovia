#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleSheet {
    pub source: String,
}

#[must_use]
pub fn parse(input: &str) -> StyleSheet {
    StyleSheet {
        source: input.to_string(),
    }
}

#[must_use]
pub fn generate(stylesheet: &StyleSheet) -> String {
    stylesheet.source.clone()
}
