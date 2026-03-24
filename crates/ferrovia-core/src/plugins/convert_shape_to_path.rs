use regex::Regex;
use serde_json::Value;

use crate::path::stringify_path_data;
use crate::types::{PathDataItem, XastChild, XastRoot};

/// Apply the `convertShapeToPath` plugin.
///
/// # Errors
///
/// This direct port currently does not return plugin-specific runtime errors.
pub fn apply(root: &mut XastRoot, params: Option<&Value>) -> crate::error::Result<()> {
    let convert_arcs = params
        .and_then(|value| value.get("convertArcs"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let precision = params
        .and_then(|value| value.get("floatPrecision"))
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok());
    let number_regex =
        Regex::new(r"[-+]?(?:\d*\.\d+|\d+\.?)(?:[eE][-+]?\d+)?").expect("valid number regex");
    convert_shape_to_path(&mut root.children, convert_arcs, precision, &number_regex);
    Ok(())
}

fn convert_shape_to_path(
    children: &mut Vec<XastChild>,
    convert_arcs: bool,
    precision: Option<usize>,
    number_regex: &Regex,
) {
    let mut index = 0usize;
    while index < children.len() {
        let mut removed = false;
        let mut remove_current = false;
        if let XastChild::Element(element) = &mut children[index] {
            if element.name == "rect"
                && element.get_attribute("width").is_some()
                && element.get_attribute("height").is_some()
                && element.get_attribute("rx").is_none()
                && element.get_attribute("ry").is_none()
            {
                let x = parse_number_or_default(element.get_attribute("x"), 0.0);
                let y = parse_number_or_default(element.get_attribute("y"), 0.0);
                let width = parse_number_or_default(element.get_attribute("width"), f64::NAN);
                let height = parse_number_or_default(element.get_attribute("height"), f64::NAN);
                if !(x - y + width - height).is_nan() {
                    let path_data = vec![
                        PathDataItem {
                            command: 'M',
                            args: vec![x, y],
                        },
                        PathDataItem {
                            command: 'H',
                            args: vec![x + width],
                        },
                        PathDataItem {
                            command: 'V',
                            args: vec![y + height],
                        },
                        PathDataItem {
                            command: 'H',
                            args: vec![x],
                        },
                        PathDataItem {
                            command: 'z',
                            args: Vec::new(),
                        },
                    ];
                    element.name = "path".to_string();
                    element.set_attribute("d", stringify_path_data(&path_data, precision));
                    element.remove_attribute("x");
                    element.remove_attribute("y");
                    element.remove_attribute("width");
                    element.remove_attribute("height");
                }
            }

            if element.name == "line" {
                let x1 = parse_number_or_default(element.get_attribute("x1"), 0.0);
                let y1 = parse_number_or_default(element.get_attribute("y1"), 0.0);
                let x2 = parse_number_or_default(element.get_attribute("x2"), 0.0);
                let y2 = parse_number_or_default(element.get_attribute("y2"), 0.0);
                if !(x1 - y1 + x2 - y2).is_nan() {
                    let path_data = vec![
                        PathDataItem {
                            command: 'M',
                            args: vec![x1, y1],
                        },
                        PathDataItem {
                            command: 'L',
                            args: vec![x2, y2],
                        },
                    ];
                    element.name = "path".to_string();
                    element.set_attribute("d", stringify_path_data(&path_data, precision));
                    element.remove_attribute("x1");
                    element.remove_attribute("y1");
                    element.remove_attribute("x2");
                    element.remove_attribute("y2");
                }
            }

            if (element.name == "polyline" || element.name == "polygon")
                && element.get_attribute("points").is_some()
            {
                let points_value = element
                    .get_attribute("points")
                    .map(str::to_string)
                    .unwrap_or_default();
                let coords = number_regex
                    .find_iter(points_value.as_str())
                    .filter_map(|capture| capture.as_str().parse::<f64>().ok())
                    .collect::<Vec<_>>();
                if coords.len() < 4 {
                    remove_current = true;
                } else {
                    let mut path_data = Vec::<PathDataItem>::new();
                    for (point_index, chunk) in coords.chunks(2).enumerate() {
                        if chunk.len() == 2 {
                            path_data.push(PathDataItem {
                                command: if point_index == 0 { 'M' } else { 'L' },
                                args: vec![chunk[0], chunk[1]],
                            });
                        }
                    }
                    if element.name == "polygon" {
                        path_data.push(PathDataItem {
                            command: 'z',
                            args: Vec::new(),
                        });
                    }
                    element.name = "path".to_string();
                    element.set_attribute("d", stringify_path_data(&path_data, precision));
                    element.remove_attribute("points");
                }
            }

            if !removed && convert_arcs && element.name == "circle" {
                let cx = parse_number_or_default(element.get_attribute("cx"), 0.0);
                let cy = parse_number_or_default(element.get_attribute("cy"), 0.0);
                let r = parse_number_or_default(element.get_attribute("r"), 0.0);
                if !(cx - cy + r).is_nan() {
                    let path_data = vec![
                        PathDataItem {
                            command: 'M',
                            args: vec![cx, cy - r],
                        },
                        PathDataItem {
                            command: 'A',
                            args: vec![r, r, 0.0, 1.0, 0.0, cx, cy + r],
                        },
                        PathDataItem {
                            command: 'A',
                            args: vec![r, r, 0.0, 1.0, 0.0, cx, cy - r],
                        },
                        PathDataItem {
                            command: 'z',
                            args: Vec::new(),
                        },
                    ];
                    element.name = "path".to_string();
                    element.set_attribute("d", stringify_path_data(&path_data, precision));
                    element.remove_attribute("cx");
                    element.remove_attribute("cy");
                    element.remove_attribute("r");
                }
            }

            if !removed && convert_arcs && element.name == "ellipse" {
                let cx = parse_number_or_default(element.get_attribute("cx"), 0.0);
                let cy = parse_number_or_default(element.get_attribute("cy"), 0.0);
                let rx = parse_number_or_default(element.get_attribute("rx"), 0.0);
                let ry = parse_number_or_default(element.get_attribute("ry"), 0.0);
                if !(cx - cy + rx - ry).is_nan() {
                    let path_data = vec![
                        PathDataItem {
                            command: 'M',
                            args: vec![cx, cy - ry],
                        },
                        PathDataItem {
                            command: 'A',
                            args: vec![rx, ry, 0.0, 1.0, 0.0, cx, cy + ry],
                        },
                        PathDataItem {
                            command: 'A',
                            args: vec![rx, ry, 0.0, 1.0, 0.0, cx, cy - ry],
                        },
                        PathDataItem {
                            command: 'z',
                            args: Vec::new(),
                        },
                    ];
                    element.name = "path".to_string();
                    element.set_attribute("d", stringify_path_data(&path_data, precision));
                    element.remove_attribute("cx");
                    element.remove_attribute("cy");
                    element.remove_attribute("rx");
                    element.remove_attribute("ry");
                }
            }

            if !removed {
                convert_shape_to_path(&mut element.children, convert_arcs, precision, number_regex);
            }
        }

        if remove_current {
            children.remove(index);
            removed = true;
        }

        if !removed {
            index += 1;
        }
    }
}

fn parse_number_or_default(value: Option<&str>, default: f64) -> f64 {
    value
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(default)
}
