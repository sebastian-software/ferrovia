use regex::{Regex, RegexBuilder};
use serde_json::Value;

use crate::svgo::tools::includes_url_reference;
use crate::types::{XastChild, XastRoot};

const COLOR_PROPS: &[&str] = &[
    "color",
    "fill",
    "flood-color",
    "lighting-color",
    "stop-color",
    "stroke",
];

const COLORS_NAMES: &[(&str, &str)] = &[
    ("aliceblue", "#f0f8ff"),
    ("antiquewhite", "#faebd7"),
    ("aqua", "#0ff"),
    ("aquamarine", "#7fffd4"),
    ("azure", "#f0ffff"),
    ("beige", "#f5f5dc"),
    ("bisque", "#ffe4c4"),
    ("black", "#000"),
    ("blanchedalmond", "#ffebcd"),
    ("blue", "#00f"),
    ("blueviolet", "#8a2be2"),
    ("brown", "#a52a2a"),
    ("burlywood", "#deb887"),
    ("cadetblue", "#5f9ea0"),
    ("chartreuse", "#7fff00"),
    ("chocolate", "#d2691e"),
    ("coral", "#ff7f50"),
    ("cornflowerblue", "#6495ed"),
    ("cornsilk", "#fff8dc"),
    ("crimson", "#dc143c"),
    ("cyan", "#0ff"),
    ("darkblue", "#00008b"),
    ("darkcyan", "#008b8b"),
    ("darkgoldenrod", "#b8860b"),
    ("darkgray", "#a9a9a9"),
    ("darkgreen", "#006400"),
    ("darkgrey", "#a9a9a9"),
    ("darkkhaki", "#bdb76b"),
    ("darkmagenta", "#8b008b"),
    ("darkolivegreen", "#556b2f"),
    ("darkorange", "#ff8c00"),
    ("darkorchid", "#9932cc"),
    ("darkred", "#8b0000"),
    ("darksalmon", "#e9967a"),
    ("darkseagreen", "#8fbc8f"),
    ("darkslateblue", "#483d8b"),
    ("darkslategray", "#2f4f4f"),
    ("darkslategrey", "#2f4f4f"),
    ("darkturquoise", "#00ced1"),
    ("darkviolet", "#9400d3"),
    ("deeppink", "#ff1493"),
    ("deepskyblue", "#00bfff"),
    ("dimgray", "#696969"),
    ("dimgrey", "#696969"),
    ("dodgerblue", "#1e90ff"),
    ("firebrick", "#b22222"),
    ("floralwhite", "#fffaf0"),
    ("forestgreen", "#228b22"),
    ("fuchsia", "#f0f"),
    ("gainsboro", "#dcdcdc"),
    ("ghostwhite", "#f8f8ff"),
    ("gold", "#ffd700"),
    ("goldenrod", "#daa520"),
    ("gray", "#808080"),
    ("green", "#008000"),
    ("greenyellow", "#adff2f"),
    ("grey", "#808080"),
    ("honeydew", "#f0fff0"),
    ("hotpink", "#ff69b4"),
    ("indianred", "#cd5c5c"),
    ("indigo", "#4b0082"),
    ("ivory", "#fffff0"),
    ("khaki", "#f0e68c"),
    ("lavender", "#e6e6fa"),
    ("lavenderblush", "#fff0f5"),
    ("lawngreen", "#7cfc00"),
    ("lemonchiffon", "#fffacd"),
    ("lightblue", "#add8e6"),
    ("lightcoral", "#f08080"),
    ("lightcyan", "#e0ffff"),
    ("lightgoldenrodyellow", "#fafad2"),
    ("lightgray", "#d3d3d3"),
    ("lightgreen", "#90ee90"),
    ("lightgrey", "#d3d3d3"),
    ("lightpink", "#ffb6c1"),
    ("lightsalmon", "#ffa07a"),
    ("lightseagreen", "#20b2aa"),
    ("lightskyblue", "#87cefa"),
    ("lightslategray", "#789"),
    ("lightslategrey", "#789"),
    ("lightsteelblue", "#b0c4de"),
    ("lightyellow", "#ffffe0"),
    ("lime", "#0f0"),
    ("limegreen", "#32cd32"),
    ("linen", "#faf0e6"),
    ("magenta", "#f0f"),
    ("maroon", "#800000"),
    ("mediumaquamarine", "#66cdaa"),
    ("mediumblue", "#0000cd"),
    ("mediumorchid", "#ba55d3"),
    ("mediumpurple", "#9370db"),
    ("mediumseagreen", "#3cb371"),
    ("mediumslateblue", "#7b68ee"),
    ("mediumspringgreen", "#00fa9a"),
    ("mediumturquoise", "#48d1cc"),
    ("mediumvioletred", "#c71585"),
    ("midnightblue", "#191970"),
    ("mintcream", "#f5fffa"),
    ("mistyrose", "#ffe4e1"),
    ("moccasin", "#ffe4b5"),
    ("navajowhite", "#ffdead"),
    ("navy", "#000080"),
    ("oldlace", "#fdf5e6"),
    ("olive", "#808000"),
    ("olivedrab", "#6b8e23"),
    ("orange", "#ffa500"),
    ("orangered", "#ff4500"),
    ("orchid", "#da70d6"),
    ("palegoldenrod", "#eee8aa"),
    ("palegreen", "#98fb98"),
    ("paleturquoise", "#afeeee"),
    ("palevioletred", "#db7093"),
    ("papayawhip", "#ffefd5"),
    ("peachpuff", "#ffdab9"),
    ("peru", "#cd853f"),
    ("pink", "#ffc0cb"),
    ("plum", "#dda0dd"),
    ("powderblue", "#b0e0e6"),
    ("purple", "#800080"),
    ("rebeccapurple", "#639"),
    ("red", "#f00"),
    ("rosybrown", "#bc8f8f"),
    ("royalblue", "#4169e1"),
    ("saddlebrown", "#8b4513"),
    ("salmon", "#fa8072"),
    ("sandybrown", "#f4a460"),
    ("seagreen", "#2e8b57"),
    ("seashell", "#fff5ee"),
    ("sienna", "#a0522d"),
    ("silver", "#c0c0c0"),
    ("skyblue", "#87ceeb"),
    ("slateblue", "#6a5acd"),
    ("slategray", "#708090"),
    ("slategrey", "#708090"),
    ("snow", "#fffafa"),
    ("springgreen", "#00ff7f"),
    ("steelblue", "#4682b4"),
    ("tan", "#d2b48c"),
    ("teal", "#008080"),
    ("thistle", "#d8bfd8"),
    ("tomato", "#ff6347"),
    ("turquoise", "#40e0d0"),
    ("violet", "#ee82ee"),
    ("wheat", "#f5deb3"),
    ("white", "#fff"),
    ("whitesmoke", "#f5f5f5"),
    ("yellow", "#ff0"),
    ("yellowgreen", "#9acd32"),
];

const COLORS_SHORT_NAMES: &[(&str, &str)] = &[
    ("#f0ffff", "azure"),
    ("#f5f5dc", "beige"),
    ("#ffe4c4", "bisque"),
    ("#a52a2a", "brown"),
    ("#ff7f50", "coral"),
    ("#ffd700", "gold"),
    ("#808080", "gray"),
    ("#008000", "green"),
    ("#4b0082", "indigo"),
    ("#fffff0", "ivory"),
    ("#f0e68c", "khaki"),
    ("#faf0e6", "linen"),
    ("#800000", "maroon"),
    ("#000080", "navy"),
    ("#808000", "olive"),
    ("#ffa500", "orange"),
    ("#da70d6", "orchid"),
    ("#cd853f", "peru"),
    ("#ffc0cb", "pink"),
    ("#dda0dd", "plum"),
    ("#800080", "purple"),
    ("#f00", "red"),
    ("#ff0000", "red"),
    ("#fa8072", "salmon"),
    ("#a0522d", "sienna"),
    ("#c0c0c0", "silver"),
    ("#fffafa", "snow"),
    ("#d2b48c", "tan"),
    ("#008080", "teal"),
    ("#ff6347", "tomato"),
    ("#ee82ee", "violet"),
    ("#f5deb3", "wheat"),
];

/// Apply the `convertColors` plugin.
///
/// # Errors
///
/// Returns an error if the configured `currentColor` regex cannot be compiled.
///
/// # Panics
///
/// Panics if the hard-coded `rgb(...)` regex becomes invalid.
pub fn apply(root: &mut XastRoot, params: Option<&Value>) -> crate::error::Result<()> {
    let current_color = parse_current_color_matcher(params.and_then(|value| value.get("currentColor")))?;
    let options = ConvertColorsOptions {
        names2hex: bool_param(params, "names2hex", true),
        rgb2hex: bool_param(params, "rgb2hex", true),
        convert_case: convert_case_param(params.and_then(|value| value.get("convertCase"))),
        shorthex: bool_param(params, "shorthex", true),
        shortname: bool_param(params, "shortname", true),
    };
    let rgb_regex = Regex::new(
        r"^rgb\(\s*([+-]?(?:\d*\.\d+|\d+\.?)%?)(?:\s*,\s*|\s+)([+-]?(?:\d*\.\d+|\d+\.?)%?)(?:\s*,\s*|\s+)([+-]?(?:\d*\.\d+|\d+\.?)%?)\s*\)$",
    )
    .expect("valid rgb regex");
    let mut mask_counter = 0usize;
    convert_colors(
        &mut root.children,
        &current_color,
        options,
        &rgb_regex,
        &mut mask_counter,
    );
    Ok(())
}

#[derive(Clone, Copy)]
enum ConvertCase {
    Lower,
    Upper,
}

enum CurrentColorMatcher {
    Disabled,
    Any,
    Exact(String),
    Regex(Regex),
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy)]
struct ConvertColorsOptions {
    names2hex: bool,
    rgb2hex: bool,
    convert_case: Option<ConvertCase>,
    shorthex: bool,
    shortname: bool,
}

fn convert_colors(
    children: &mut Vec<XastChild>,
    current_color: &CurrentColorMatcher,
    options: ConvertColorsOptions,
    rgb_regex: &Regex,
    mask_counter: &mut usize,
) {
    for child in children {
        let XastChild::Element(element) = child else {
            continue;
        };

        let entered_mask = element.name == "mask";
        if entered_mask {
            *mask_counter += 1;
        }

        for attribute in &mut element.attributes {
            if !is_color_prop(attribute.name.as_str()) {
                continue;
            }

            let mut value = attribute.value.clone();

            if *mask_counter == 0 && current_color_matches(current_color, value.as_str()) {
                value = "currentColor".to_string();
            }

            if options.names2hex {
                let color_name = value.to_lowercase();
                if let Some(hex) = color_name_to_hex(color_name.as_str()) {
                    value = hex.to_string();
                }
            }

            if options.rgb2hex
                && let Some(converted) = convert_rgb_to_hex(value.as_str(), rgb_regex)
            {
                value = converted;
            }

            if let Some(case_mode) = options.convert_case
                && !includes_url_reference(value.as_str())
                && value != "currentColor"
            {
                value = match case_mode {
                    ConvertCase::Lower => value.to_lowercase(),
                    ConvertCase::Upper => value.to_uppercase(),
                };
            }

            if options.shorthex
                && let Some(shortened) = shorten_hex(value.as_str())
            {
                value = shortened;
            }

            if options.shortname {
                let color_name = value.to_lowercase();
                if let Some(short_name) = hex_to_short_name(color_name.as_str()) {
                    value = short_name.to_string();
                }
            }

            attribute.value = value;
        }

        convert_colors(
            &mut element.children,
            current_color,
            options,
            rgb_regex,
            mask_counter,
        );

        if entered_mask {
            *mask_counter -= 1;
        }
    }
}

fn parse_current_color_matcher(
    value: Option<&Value>,
) -> Result<CurrentColorMatcher, crate::error::FerroviaError> {
    let Some(value) = value else {
        return Ok(CurrentColorMatcher::Disabled);
    };

    match value {
        Value::Bool(true) => Ok(CurrentColorMatcher::Any),
        Value::String(value) => {
            if let Some((pattern, flags)) = parse_regex_literal(value) {
                let regex = build_regex(pattern, flags)?;
                Ok(CurrentColorMatcher::Regex(regex))
            } else {
                Ok(CurrentColorMatcher::Exact(value.clone()))
            }
        }
        Value::Object(object) => {
            let Some(pattern) = object.get("regex").and_then(Value::as_str) else {
                return Ok(CurrentColorMatcher::Disabled);
            };
            let flags = object.get("flags").and_then(Value::as_str).unwrap_or_default();
            let regex = build_regex(pattern, flags)?;
            Ok(CurrentColorMatcher::Regex(regex))
        }
        _ => Ok(CurrentColorMatcher::Disabled),
    }
}

fn current_color_matches(matcher: &CurrentColorMatcher, value: &str) -> bool {
    match matcher {
        CurrentColorMatcher::Disabled => false,
        CurrentColorMatcher::Any => value != "none",
        CurrentColorMatcher::Exact(expected) => value == expected,
        CurrentColorMatcher::Regex(regex) => regex.is_match(value),
    }
}

fn parse_regex_literal(value: &str) -> Option<(&str, &str)> {
    if !value.starts_with('/') {
        return None;
    }
    let last_slash = value.rfind('/')?;
    if last_slash == 0 {
        return None;
    }
    Some((&value[1..last_slash], &value[last_slash + 1..]))
}

fn build_regex(pattern: &str, flags: &str) -> Result<Regex, crate::error::FerroviaError> {
    let mut builder = RegexBuilder::new(pattern);
    if flags.contains('i') {
        builder.case_insensitive(true);
    }
    builder
        .build()
        .map_err(|error| crate::error::FerroviaError::InvalidConfig(error.to_string()))
}

fn convert_case_param(value: Option<&Value>) -> Option<ConvertCase> {
    match value.and_then(Value::as_str) {
        Some("lower") | None => Some(ConvertCase::Lower),
        Some("upper") => Some(ConvertCase::Upper),
        Some(_) => None,
    }
}

fn bool_param(params: Option<&Value>, name: &str, default: bool) -> bool {
    params
        .and_then(|value| value.get(name))
        .and_then(Value::as_bool)
        .unwrap_or(default)
}

fn is_color_prop(name: &str) -> bool {
    COLOR_PROPS.contains(&name)
}

fn color_name_to_hex(name: &str) -> Option<&'static str> {
    COLORS_NAMES
        .iter()
        .find_map(|(candidate, hex)| (*candidate == name).then_some(*hex))
}

fn hex_to_short_name(value: &str) -> Option<&'static str> {
    COLORS_SHORT_NAMES
        .iter()
        .find_map(|(hex, name)| (*hex == value).then_some(*name))
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn convert_rgb_to_hex(value: &str, regex: &Regex) -> Option<String> {
    let captures = regex.captures(value)?;
    let mut numbers = Vec::<u8>::new();
    for index in 1..=3 {
        let matched = captures.get(index)?.as_str();
        let number = if matched.contains('%') {
            f64::from((matched.trim_end_matches('%').parse::<f32>().ok()? * 2.55_f32).round())
        } else {
            matched.parse::<f64>().ok()?
        };
        let clamped = number.clamp(0.0, 255.0);
        numbers.push(clamped as u8);
    }

    Some(convert_rgb_triplet_to_hex(
        numbers[0], numbers[1], numbers[2],
    ))
}

fn convert_rgb_triplet_to_hex(red: u8, green: u8, blue: u8) -> String {
    format!("#{red:02X}{green:02X}{blue:02X}")
}

fn shorten_hex(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    if bytes.len() != 7 || bytes[0] != b'#' {
        return None;
    }
    if !bytes[1].is_ascii_hexdigit()
        || !bytes[2].is_ascii_hexdigit()
        || !bytes[3].is_ascii_hexdigit()
        || !bytes[4].is_ascii_hexdigit()
        || !bytes[5].is_ascii_hexdigit()
        || !bytes[6].is_ascii_hexdigit()
    {
        return None;
    }
    if !bytes[1].eq_ignore_ascii_case(&bytes[2])
        || !bytes[3].eq_ignore_ascii_case(&bytes[4])
        || !bytes[5].eq_ignore_ascii_case(&bytes[6])
    {
        return None;
    }
    Some(format!(
        "#{}{}{}",
        bytes[1] as char, bytes[3] as char, bytes[5] as char
    ))
}
