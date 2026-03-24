use crate::plugins::_path::{js2path, path2js};
use crate::plugins::_transforms::{transform_arc, transform2js, transforms_multiply};
use crate::style::{collect_stylesheet, compute_style};
use crate::svgo::tools::includes_url_reference;
use crate::types::{
    ComputedStyle, Js2PathParams, PathDataItem, Stylesheet, XastChild, XastElement, XastRoot,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyTransformsParams {
    pub transform_precision: usize,
    pub apply_transforms_stroked: bool,
}

impl Default for ApplyTransformsParams {
    fn default() -> Self {
        Self {
            transform_precision: 5,
            apply_transforms_stroked: false,
        }
    }
}

pub fn apply_transforms(root: &mut XastRoot, params: &ApplyTransformsParams) {
    let stylesheet = collect_stylesheet(root);
    apply_to_children(&mut root.children, &stylesheet, params);
}

fn apply_to_children(
    children: &mut [XastChild],
    stylesheet: &Stylesheet,
    params: &ApplyTransformsParams,
) {
    for child in children {
        if let XastChild::Element(element) = child {
            apply_to_element(element, stylesheet, params);
            apply_to_children(&mut element.children, stylesheet, params);
        }
    }
}

fn apply_to_element(
    node: &mut XastElement,
    stylesheet: &Stylesheet,
    params: &ApplyTransformsParams,
) {
    if node.get_attribute("d").is_none() || node.get_attribute("id").is_some() {
        return;
    }

    let Some(transform_attribute) = node.get_attribute("transform") else {
        return;
    };
    if transform_attribute.is_empty() || node.get_attribute("style").is_some() {
        return;
    }

    if node.attributes.iter().any(|attribute| {
        (attribute.name == "fill"
            || attribute.name == "stroke"
            || attribute.name == "href"
            || attribute.name.ends_with(":href")
            || attribute.name.ends_with("path")
            || attribute.name.ends_with("filter"))
            && includes_url_reference(attribute.value.as_str())
    }) {
        return;
    }

    let computed_style = compute_style(stylesheet, node);
    if transform_overridden(
        transform_attribute,
        computed_style
            .iter()
            .find(|(name, _)| name == "transform")
            .map(|(_, value)| value),
    ) {
        return;
    }

    if stroke_is_dynamic(&computed_style) {
        return;
    }

    let matrix = transforms_multiply(&transform2js(transform_attribute));
    if matrix.data.len() != 6 {
        return;
    }

    let mut path_data = path2js(node);
    apply_matrix_to_path_data(&mut path_data, &matrix.data, params.transform_precision);
    js2path(node, &path_data, &Js2PathParams::default());
    node.remove_attribute("transform");
}

fn transform_overridden(transform_attribute: &str, computed: Option<&ComputedStyle>) -> bool {
    match computed {
        Some(ComputedStyle::Static { value, .. }) => value != transform_attribute,
        Some(ComputedStyle::Dynamic { .. }) => true,
        None => false,
    }
}

fn stroke_is_dynamic(computed_style: &[(String, ComputedStyle)]) -> bool {
    computed_style.iter().any(|(name, value)| {
        (name == "stroke" || name == "stroke-width")
            && matches!(value, ComputedStyle::Dynamic { .. })
    })
}

fn transform_absolute_point(matrix: &[f64], x: f64, y: f64) -> [f64; 2] {
    [
        matrix[0].mul_add(x, matrix[2].mul_add(y, matrix[4])),
        matrix[1].mul_add(x, matrix[3].mul_add(y, matrix[5])),
    ]
}

fn transform_relative_point(matrix: &[f64], x: f64, y: f64) -> [f64; 2] {
    [
        matrix[0].mul_add(x, matrix[2] * y),
        matrix[1].mul_add(x, matrix[3] * y),
    ]
}

#[allow(clippy::too_many_lines)]
fn apply_matrix_to_path_data(
    path_data: &mut [PathDataItem],
    matrix: &[f64],
    transform_precision: usize,
) {
    let _ = transform_precision;
    let mut start = [0.0f64, 0.0];
    let mut cursor = [0.0f64, 0.0];

    for path_item in path_data {
        let mut command = path_item.command;
        let mut args = path_item.args.clone();

        match command {
            'M' => {
                cursor = [args[0], args[1]];
                start = cursor;
                let next = transform_absolute_point(matrix, args[0], args[1]);
                args[0] = next[0];
                args[1] = next[1];
            }
            'm' => {
                cursor[0] += args[0];
                cursor[1] += args[1];
                start = cursor;
                let next = transform_relative_point(matrix, args[0], args[1]);
                args[0] = next[0];
                args[1] = next[1];
            }
            'H' => {
                command = 'L';
                args = vec![args[0], cursor[1]];
                cursor = [args[0], args[1]];
                let next = transform_absolute_point(matrix, args[0], args[1]);
                args[0] = next[0];
                args[1] = next[1];
            }
            'h' => {
                command = 'l';
                args = vec![args[0], 0.0];
                cursor[0] += args[0];
                let next = transform_relative_point(matrix, args[0], 0.0);
                args[0] = next[0];
                args[1] = next[1];
            }
            'V' => {
                command = 'L';
                args = vec![cursor[0], args[0]];
                cursor = [args[0], args[1]];
                let next = transform_absolute_point(matrix, args[0], args[1]);
                args[0] = next[0];
                args[1] = next[1];
            }
            'v' => {
                command = 'l';
                args = vec![0.0, args[0]];
                cursor[1] += args[1];
                let next = transform_relative_point(matrix, 0.0, args[1]);
                args[0] = next[0];
                args[1] = next[1];
            }
            'L' | 'T' => {
                cursor = [args[0], args[1]];
                let next = transform_absolute_point(matrix, args[0], args[1]);
                args[0] = next[0];
                args[1] = next[1];
            }
            'l' | 't' => {
                cursor[0] += args[0];
                cursor[1] += args[1];
                let next = transform_relative_point(matrix, args[0], args[1]);
                args[0] = next[0];
                args[1] = next[1];
            }
            'C' => {
                cursor = [args[4], args[5]];
                let first = transform_absolute_point(matrix, args[0], args[1]);
                let second = transform_absolute_point(matrix, args[2], args[3]);
                let end = transform_absolute_point(matrix, args[4], args[5]);
                args[0] = first[0];
                args[1] = first[1];
                args[2] = second[0];
                args[3] = second[1];
                args[4] = end[0];
                args[5] = end[1];
            }
            'c' => {
                cursor[0] += args[4];
                cursor[1] += args[5];
                let first = transform_relative_point(matrix, args[0], args[1]);
                let second = transform_relative_point(matrix, args[2], args[3]);
                let end = transform_relative_point(matrix, args[4], args[5]);
                args[0] = first[0];
                args[1] = first[1];
                args[2] = second[0];
                args[3] = second[1];
                args[4] = end[0];
                args[5] = end[1];
            }
            'S' | 'Q' => {
                cursor = [args[2], args[3]];
                let second = transform_absolute_point(matrix, args[0], args[1]);
                let end = transform_absolute_point(matrix, args[2], args[3]);
                args[0] = second[0];
                args[1] = second[1];
                args[2] = end[0];
                args[3] = end[1];
            }
            's' | 'q' => {
                cursor[0] += args[2];
                cursor[1] += args[3];
                let second = transform_relative_point(matrix, args[0], args[1]);
                let end = transform_relative_point(matrix, args[2], args[3]);
                args[0] = second[0];
                args[1] = second[1];
                args[2] = end[0];
                args[3] = end[1];
            }
            'A' | 'a' => {
                if command == 'A' {
                    cursor = [args[5], args[6]];
                } else {
                    cursor[0] += args[5];
                    cursor[1] += args[6];
                }
                transform_arc(&mut args, matrix);
            }
            'z' | 'Z' => {
                cursor = start;
                command = 'z';
            }
            _ => {}
        }

        path_item.command = command;
        path_item.args = args;
    }
}
