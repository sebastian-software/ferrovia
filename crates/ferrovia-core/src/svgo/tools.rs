use regex::Regex;

use crate::plugins::_collections::is_reference_prop;

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

#[must_use]
pub fn includes_url_reference(body: &str) -> bool {
    Regex::new(r#"url\((?:"|')?#([^)"']+)(?:"|')?\)"#)
        .is_ok_and(|regex| regex.is_match(body))
}

#[must_use]
pub fn find_references(attribute: &str, value: &str) -> Vec<String> {
    let mut results = Vec::new();

    if is_reference_prop(attribute)
        && let Ok(regex) = Regex::new(r#"url\((?:"|')?#([^)"']+)(?:"|')?\)"#)
    {
        for captures in regex.captures_iter(value) {
            if let Some(id) = captures.get(1) {
                results.push(id.as_str().to_string());
            }
        }
    }

    if (attribute == "href" || attribute.ends_with(":href"))
        && let Some(id) = value.strip_prefix('#')
    {
        results.push(id.to_string());
    }

    if attribute == "begin"
        && let Ok(regex) = Regex::new(r"(\w+)\.[a-zA-Z]")
        && let Some(captures) = regex.captures(value)
        && let Some(id) = captures.get(1)
    {
        results.push(id.as_str().to_string());
    }

    results
}

#[must_use]
pub fn cleanup_out_data(data: &[f64], no_space_after_flags: bool) -> String {
    let mut out = String::new();
    let mut previous = 0.0f64;
    for (index, item) in data.iter().enumerate() {
        let mut delimiter = if index == 0 { "" } else { " " };
        let item_str = remove_leading_zero(*item);
        if no_space_after_flags && index > 0 && index % 7 == 5 {
            delimiter = "";
        }
        if delimiter == " " && (*item < 0.0 || (item_str.starts_with('.') && previous.fract() != 0.0)) {
            delimiter = "";
        }
        out.push_str(delimiter);
        out.push_str(item_str.as_str());
        previous = *item;
    }
    out
}

#[must_use]
pub fn remove_leading_zero(value: f64) -> String {
    let str_value = value.to_string();
    if 0.0 < value && value < 1.0 && str_value.starts_with('0') {
        return str_value[1..].to_string();
    }
    if -1.0 < value && value < 0.0 && str_value.as_bytes().get(1) == Some(&b'0') {
        return format!("-{}", &str_value[2..]);
    }
    str_value
}

#[must_use]
pub fn to_fixed(num: f64, precision: usize) -> f64 {
    let exponent = i32::try_from(precision).unwrap_or(i32::MAX);
    let pow = 10f64.powi(exponent);
    (num * pow).round() / pow
}
