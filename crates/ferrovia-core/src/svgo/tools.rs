use regex::Regex;

#[must_use]
pub fn has_scripts(svg_name: &str) -> bool {
    svg_name == "script"
}

#[must_use]
pub fn preserve_comment(value: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|pattern| {
        Regex::new(pattern)
            .is_ok_and(|regex| regex.is_match(value))
    })
}
