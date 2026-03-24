#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Usage {
    pub tags: Vec<String>,
    pub ids: Vec<String>,
    pub classes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MinifyOptions {
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinifyResult {
    pub css: String,
}

#[must_use]
pub fn minify(input: &str, _options: &MinifyOptions) -> MinifyResult {
    MinifyResult {
        css: minify_css(input),
    }
}

#[must_use]
pub fn minify_block(input: &str, _options: &MinifyOptions) -> MinifyResult {
    MinifyResult {
        css: minify_css(input),
    }
}

fn minify_css(input: &str) -> String {
    let without_comments = strip_comments(input);
    let mut minified = String::new();
    let mut last_was_space = false;

    for ch in without_comments.chars() {
        if ch.is_ascii_whitespace() {
            if !last_was_space {
                minified.push(' ');
                last_was_space = true;
            }
            continue;
        }
        if matches!(ch, '{' | '}' | ':' | ';' | ',') {
            if minified.ends_with(' ') {
                minified.pop();
            }
            minified.push(ch);
            last_was_space = false;
            continue;
        }
        if ch == '(' {
            if minified.ends_with(' ') {
                minified.pop();
            }
            minified.push(ch);
            last_was_space = false;
            continue;
        }
        if ch == ')' {
            if minified.ends_with(' ') {
                minified.pop();
            }
            minified.push(ch);
            last_was_space = false;
            continue;
        }
        minified.push(ch);
        last_was_space = false;
    }

    minified = minified.trim().to_string();
    for (from, to) in [
        ("{ ", "{"),
        (" }", "}"),
        (": ", ":"),
        (" ;", ";"),
        ("; ", ";"),
        (", ", ","),
        ("( ", "("),
        (" )", ")"),
    ] {
        while minified.contains(from) {
            minified = minified.replace(from, to);
        }
    }
    while minified.contains(";}") {
        minified = minified.replace(";}", "}");
    }
    if minified.ends_with(';') {
        minified.pop();
    }
    minified
}

fn strip_comments(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if index + 1 < bytes.len() && bytes[index] == b'/' && bytes[index + 1] == b'*' {
            index += 2;
            while index + 1 < bytes.len() && !(bytes[index] == b'*' && bytes[index + 1] == b'/') {
                index += 1;
            }
            index = (index + 2).min(bytes.len());
            continue;
        }
        out.push(bytes[index] as char);
        index += 1;
    }
    out
}
