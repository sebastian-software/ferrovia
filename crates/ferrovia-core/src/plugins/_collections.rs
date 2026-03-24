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

const CONTAINER_ELEMS: &[&str] = &[
    "a",
    "defs",
    "foreignObject",
    "g",
    "marker",
    "mask",
    "missing-glyph",
    "pattern",
    "svg",
    "switch",
    "symbol",
];

const DEPRECATED_GROUP_ANIMATION_ATTRIBUTE_TARGET_UNSAFE: &[&str] = &["attributeType"];
const DEPRECATED_GROUP_CONDITIONAL_PROCESSING_UNSAFE: &[&str] = &["requiredFeatures"];
const DEPRECATED_GROUP_CORE_UNSAFE: &[&str] = &["xml:base", "xml:lang", "xml:space"];
const DEPRECATED_GROUP_PRESENTATION_UNSAFE: &[&str] = &[
    "clip",
    "color-profile",
    "enable-background",
    "glyph-orientation-horizontal",
    "glyph-orientation-vertical",
    "kerning",
];
const ANIMATION_EVENT_ATTRS: &[&str] = &["onbegin", "onend", "onrepeat", "onload"];
const DOCUMENT_EVENT_ATTRS: &[&str] = &[
    "onabort", "onerror", "onresize", "onscroll", "onunload", "onzoom",
];
const DOCUMENT_ELEMENT_EVENT_ATTRS: &[&str] = &["oncopy", "oncut", "onpaste"];
const GLOBAL_EVENT_ATTRS: &[&str] = &[
    "oncancel",
    "oncanplay",
    "oncanplaythrough",
    "onchange",
    "onclick",
    "onclose",
    "oncuechange",
    "ondblclick",
    "ondrag",
    "ondragend",
    "ondragenter",
    "ondragleave",
    "ondragover",
    "ondragstart",
    "ondrop",
    "ondurationchange",
    "onemptied",
    "onended",
    "onerror",
    "onfocus",
    "oninput",
    "oninvalid",
    "onkeydown",
    "onkeypress",
    "onkeyup",
    "onload",
    "onloadeddata",
    "onloadedmetadata",
    "onloadstart",
    "onmousedown",
    "onmouseenter",
    "onmouseleave",
    "onmousemove",
    "onmouseout",
    "onmouseover",
    "onmouseup",
    "onmousewheel",
    "onpause",
    "onplay",
    "onplaying",
    "onprogress",
    "onratechange",
    "onreset",
    "onresize",
    "onscroll",
    "onseeked",
    "onseeking",
    "onselect",
    "onshow",
    "onstalled",
    "onsubmit",
    "onsuspend",
    "ontimeupdate",
    "ontoggle",
    "onvolumechange",
    "onwaiting",
];
const GRAPHICAL_EVENT_ATTRS: &[&str] = &[
    "onactivate",
    "onclick",
    "onfocusin",
    "onfocusout",
    "onload",
    "onmousedown",
    "onmousemove",
    "onmouseout",
    "onmouseover",
    "onmouseup",
];

const ATTR_GROUPS_CORE_PRESENTATION: &[&str] = &[
    "conditionalProcessing",
    "core",
    "graphicalEvent",
    "presentation",
];
const ATTR_GROUPS_CORE_PRESENTATION_XLINK: &[&str] = &[
    "conditionalProcessing",
    "core",
    "graphicalEvent",
    "presentation",
    "xlink",
];
const ATTR_GROUPS_SVG: &[&str] = &[
    "conditionalProcessing",
    "core",
    "documentEvent",
    "graphicalEvent",
    "presentation",
];

#[derive(Clone, Copy)]
pub struct DeprecatedAttrs {
    pub safe: &'static [&'static str],
    pub unsafe_attrs: &'static [&'static str],
}

#[derive(Clone, Copy)]
pub struct ElementDeprecatedConfig {
    pub attrs_groups: &'static [&'static str],
    pub deprecated: Option<DeprecatedAttrs>,
}

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

#[must_use]
pub fn is_container_elem(name: &str) -> bool {
    CONTAINER_ELEMS.contains(&name)
}

#[must_use]
pub fn deprecated_attrs_group(name: &str) -> Option<DeprecatedAttrs> {
    match name {
        "animationAttributeTarget" => Some(DeprecatedAttrs {
            safe: &[],
            unsafe_attrs: DEPRECATED_GROUP_ANIMATION_ATTRIBUTE_TARGET_UNSAFE,
        }),
        "conditionalProcessing" => Some(DeprecatedAttrs {
            safe: &[],
            unsafe_attrs: DEPRECATED_GROUP_CONDITIONAL_PROCESSING_UNSAFE,
        }),
        "core" => Some(DeprecatedAttrs {
            safe: &[],
            unsafe_attrs: DEPRECATED_GROUP_CORE_UNSAFE,
        }),
        "presentation" => Some(DeprecatedAttrs {
            safe: &[],
            unsafe_attrs: DEPRECATED_GROUP_PRESENTATION_UNSAFE,
        }),
        _ => None,
    }
}

#[must_use]
pub fn deprecated_elem_config(name: &str) -> Option<ElementDeprecatedConfig> {
    match name {
        "g" | "text" => Some(ElementDeprecatedConfig {
            attrs_groups: ATTR_GROUPS_CORE_PRESENTATION,
            deprecated: None,
        }),
        "svg" => Some(ElementDeprecatedConfig {
            attrs_groups: ATTR_GROUPS_SVG,
            deprecated: Some(DeprecatedAttrs {
                safe: &["version"],
                unsafe_attrs: &[
                    "baseProfile",
                    "contentScriptType",
                    "contentStyleType",
                    "zoomAndPan",
                ],
            }),
        }),
        "use" | "image" => Some(ElementDeprecatedConfig {
            attrs_groups: ATTR_GROUPS_CORE_PRESENTATION_XLINK,
            deprecated: None,
        }),
        "glyph" => Some(ElementDeprecatedConfig {
            attrs_groups: &["core", "presentation"],
            deprecated: Some(DeprecatedAttrs {
                safe: &[],
                unsafe_attrs: &[
                    "arabic-form",
                    "glyph-name",
                    "horiz-adv-x",
                    "orientation",
                    "unicode",
                    "vert-adv-y",
                    "vert-origin-x",
                    "vert-origin-y",
                ],
            }),
        }),
        "glyphRef" => Some(ElementDeprecatedConfig {
            attrs_groups: &["core", "presentation"],
            deprecated: Some(DeprecatedAttrs {
                safe: &[],
                unsafe_attrs: &[
                    "horiz-adv-x",
                    "vert-adv-y",
                    "vert-origin-x",
                    "vert-origin-y",
                ],
            }),
        }),
        "view" => Some(ElementDeprecatedConfig {
            attrs_groups: &["core"],
            deprecated: Some(DeprecatedAttrs {
                safe: &[],
                unsafe_attrs: &["viewTarget", "zoomAndPan"],
            }),
        }),
        "color-profile" => Some(ElementDeprecatedConfig {
            attrs_groups: &["core", "xlink"],
            deprecated: Some(DeprecatedAttrs {
                safe: &[],
                unsafe_attrs: &["name"],
            }),
        }),
        _ => None,
    }
}

#[must_use]
pub fn is_event_attr(name: &str) -> bool {
    ANIMATION_EVENT_ATTRS.contains(&name)
        || DOCUMENT_EVENT_ATTRS.contains(&name)
        || DOCUMENT_ELEMENT_EVENT_ATTRS.contains(&name)
        || GLOBAL_EVENT_ATTRS.contains(&name)
        || GRAPHICAL_EVENT_ATTRS.contains(&name)
}
