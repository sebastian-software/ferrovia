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

const EDITOR_NAMESPACES: &[&str] = &[
    "http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd",
    "http://inkscape.sourceforge.net/DTD/sodipodi-0.dtd",
    "http://www.inkscape.org/namespaces/inkscape",
    "http://ns.adobe.com/AdobeIllustrator/10.0/",
    "http://ns.adobe.com/Graphs/1.0/",
    "http://ns.adobe.com/Variables/1.0/",
    "http://ns.adobe.com/SaveForWeb/1.0/",
    "http://ns.adobe.com/Extensibility/1.0/",
    "http://ns.adobe.com/Flows/1.0/",
    "http://ns.adobe.com/ImageReplacement/1.0/",
    "http://ns.adobe.com/GenericCustomNamespace/1.0/",
    "http://ns.adobe.com/XPath/1.0/",
];

const CONDITIONAL_PROCESSING_ATTRS: &[&str] =
    &["requiredFeatures", "requiredExtensions", "systemLanguage"];

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

#[must_use]
pub fn is_editor_namespace(namespace: &str) -> bool {
    EDITOR_NAMESPACES.contains(&namespace)
}

#[must_use]
pub fn is_conditional_processing_attr(name: &str) -> bool {
    CONDITIONAL_PROCESSING_ATTRS.contains(&name)
}
