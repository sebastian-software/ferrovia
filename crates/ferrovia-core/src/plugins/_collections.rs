const TEXT_ELEMS: &[&str] = &["text", "tspan", "tref", "textPath", "altGlyph"];

const REFERENCES_PROPS: &[&str] = &[
    "fill",
    "filter",
    "stroke",
    "marker-start",
    "marker-mid",
    "marker-end",
    "clip-path",
    "mask",
    "style",
];

const PRESENTATION_ATTRS: &[&str] = &[
    "fill",
    "fill-opacity",
    "stroke",
    "stroke-width",
    "stroke-opacity",
    "opacity",
    "display",
    "visibility",
    "marker-start",
    "marker-mid",
    "marker-end",
    "filter",
    "clip-path",
    "mask",
];

#[must_use]
pub fn is_text_elem(name: &str) -> bool {
    TEXT_ELEMS.contains(&name)
}

#[must_use]
pub fn is_reference_prop(name: &str) -> bool {
    REFERENCES_PROPS.contains(&name)
}

#[must_use]
pub fn is_presentation_attr(name: &str) -> bool {
    PRESENTATION_ATTRS.contains(&name)
}
