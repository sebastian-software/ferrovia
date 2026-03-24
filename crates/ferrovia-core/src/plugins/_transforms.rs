use regex::Regex;

use crate::svgo::tools::{cleanup_out_data, to_fixed};
use crate::types::{TransformItem, TransformParams};

#[must_use]
pub fn transform2js(transform_string: &str) -> Vec<TransformItem> {
    let Ok(reg_numeric_values) = Regex::new(r"[-+]?(?:\d*\.\d+|\d+\.?)(?:[eE][-+]?\d+)?") else {
        return Vec::new();
    };
    let Ok(reg_transform) =
        Regex::new(r"(matrix|translate|scale|rotate|skewX|skewY)\s*\(\s*([^)]*?)\s*\)")
    else {
        return Vec::new();
    };
    let mut transforms = Vec::<TransformItem>::new();
    for captures in reg_transform.captures_iter(transform_string) {
        let Some(name) = captures.get(1).map(|value| value.as_str()) else {
            continue;
        };
        let Some(body) = captures.get(2).map(|value| value.as_str()) else {
            continue;
        };
        let mut transform = TransformItem {
            name: name.to_string(),
            data: Vec::new(),
        };
        for capture in reg_numeric_values.find_iter(body) {
            if let Ok(number) = capture.as_str().parse::<f64>() {
                transform.data.push(number);
            }
        }
        if !transform.data.is_empty() {
            transforms.push(transform);
        }
    }
    transforms
}

#[must_use]
pub fn transforms_multiply(transforms: &[TransformItem]) -> TransformItem {
    let mut matrix = [1.0f64, 0.0, 0.0, 1.0, 0.0, 0.0];
    for transform in transforms {
        matrix = multiply_transform_matrices(&matrix, &transform_to_matrix(transform));
    }
    TransformItem {
        name: "matrix".to_string(),
        data: matrix.to_vec(),
    }
}

#[must_use]
pub fn matrix_to_transform(
    orig_matrix: &TransformItem,
    params: &TransformParams,
) -> Vec<TransformItem> {
    let rounded = round_transform(orig_matrix.clone(), params);
    vec![rounded]
}

#[must_use]
pub fn js2transform(transforms: &[TransformItem], params: &TransformParams) -> String {
    let mut out = String::new();
    for transform in transforms {
        let rounded = round_transform(transform.clone(), params);
        out.push_str(rounded.name.as_str());
        out.push('(');
        out.push_str(cleanup_out_data(&rounded.data, false).as_str());
        out.push(')');
    }
    out
}

pub fn transform_arc(data: &mut [f64], matrix: &[f64]) {
    if data.len() != 7 || matrix.len() != 6 {
        return;
    }
    let scale_x = f64::hypot(matrix[0], matrix[1]);
    let scale_y = f64::hypot(matrix[2], matrix[3]);
    data[0] *= scale_x;
    data[1] *= scale_y;
    let x = matrix[0].mul_add(data[5], matrix[2].mul_add(data[6], matrix[4]));
    let y = matrix[1].mul_add(data[5], matrix[3].mul_add(data[6], matrix[5]));
    data[5] = x;
    data[6] = y;
}

fn round_transform(mut transform: TransformItem, params: &TransformParams) -> TransformItem {
    let precision = if transform.name == "rotate" || transform.name.starts_with("skew") {
        params.deg_precision.unwrap_or(params.float_precision)
    } else if transform.name == "matrix" {
        params.transform_precision
    } else {
        params.float_precision
    };
    transform.data = transform
        .data
        .into_iter()
        .map(|value| to_fixed(value, precision))
        .collect();
    transform
}

fn transform_to_matrix(transform: &TransformItem) -> [f64; 6] {
    if transform.name == "matrix" && transform.data.len() == 6 {
        return [
            transform.data[0],
            transform.data[1],
            transform.data[2],
            transform.data[3],
            transform.data[4],
            transform.data[5],
        ];
    }

    match transform.name.as_str() {
        "translate" => [
            1.0,
            0.0,
            0.0,
            1.0,
            transform.data[0],
            *transform.data.get(1).unwrap_or(&0.0),
        ],
        "scale" => [
            transform.data[0],
            0.0,
            0.0,
            *transform.data.get(1).unwrap_or(&transform.data[0]),
            0.0,
            0.0,
        ],
        "rotate" => {
            let angle = transform.data[0].to_radians();
            let cos = angle.cos();
            let sin = angle.sin();
            let cx = *transform.data.get(1).unwrap_or(&0.0);
            let cy = *transform.data.get(2).unwrap_or(&0.0);
            [
                cos,
                sin,
                -sin,
                cos,
                cx - cx * cos + cy * sin,
                cy - cx * sin - cy * cos,
            ]
        }
        "skewX" => [
            1.0,
            0.0,
            transform.data[0].to_radians().tan(),
            1.0,
            0.0,
            0.0,
        ],
        "skewY" => [
            1.0,
            transform.data[0].to_radians().tan(),
            0.0,
            1.0,
            0.0,
            0.0,
        ],
        _ => [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
    }
}

fn multiply_transform_matrices(left: &[f64; 6], right: &[f64; 6]) -> [f64; 6] {
    [
        left[0].mul_add(right[0], left[2] * right[1]),
        left[1].mul_add(right[0], left[3] * right[1]),
        left[0].mul_add(right[2], left[2] * right[3]),
        left[1].mul_add(right[2], left[3] * right[3]),
        left[0].mul_add(right[4], left[2].mul_add(right[5], left[4])),
        left[1].mul_add(right[4], left[3].mul_add(right[5], left[5])),
    ]
}
