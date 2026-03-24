use crate::path::{parse_path_data, stringify_path_data};
use crate::types::{Js2PathParams, PathDataItem, XastElement};

#[must_use]
pub fn path2js(path: &XastElement) -> Vec<PathDataItem> {
    let Some(path_data) = path.get_attribute("d") else {
        return Vec::new();
    };
    let mut parsed = parse_path_data(path_data);
    if let Some(first) = parsed.first_mut()
        && first.command == 'm'
    {
        first.command = 'M';
    }
    parsed
}

pub fn js2path(path: &mut XastElement, data: &[PathDataItem], params: &Js2PathParams) {
    let mut path_data = Vec::<PathDataItem>::new();
    for item in data {
        if !path_data.is_empty() && matches!(item.command, 'M' | 'm') {
            let last = path_data.last();
            if last.is_some_and(|last| matches!(last.command, 'M' | 'm')) {
                path_data.pop();
            }
        }
        path_data.push(PathDataItem {
            command: item.command,
            args: item.args.clone(),
        });
    }
    let stringified = stringify_path_data(&path_data, params.float_precision);
    path.set_attribute("d", stringified);
}

#[must_use]
pub fn intersects(path1: &[PathDataItem], path2: &[PathDataItem]) -> bool {
    let points1 = gather_points(&convert_relative_to_absolute(path1));
    let points2 = gather_points(&convert_relative_to_absolute(path2));
    if points1.is_empty() || points2.is_empty() {
        return false;
    }
    let Some(bounds1) = compute_bounds(&points1) else {
        return false;
    };
    let Some(bounds2) = compute_bounds(&points2) else {
        return false;
    };
    !(bounds1.max_x <= bounds2.min_x
        || bounds2.max_x <= bounds1.min_x
        || bounds1.max_y <= bounds2.min_y
        || bounds2.max_y <= bounds1.min_y)
}

fn convert_relative_to_absolute(data: &[PathDataItem]) -> Vec<PathDataItem> {
    let mut new_data = Vec::<PathDataItem>::new();
    let mut start = [0.0f64, 0.0];
    let mut cursor = [0.0f64, 0.0];

    for item in data {
        let mut command = item.command;
        let mut args = item.args.clone();

        match command {
            'm' => {
                args[0] += cursor[0];
                args[1] += cursor[1];
                command = 'M';
            }
            'h' => {
                args[0] += cursor[0];
                command = 'H';
            }
            'v' => {
                args[0] += cursor[1];
                command = 'V';
            }
            'l' => {
                args[0] += cursor[0];
                args[1] += cursor[1];
                command = 'L';
            }
            'c' => {
                args[0] += cursor[0];
                args[1] += cursor[1];
                args[2] += cursor[0];
                args[3] += cursor[1];
                args[4] += cursor[0];
                args[5] += cursor[1];
                command = 'C';
            }
            's' => {
                args[0] += cursor[0];
                args[1] += cursor[1];
                args[2] += cursor[0];
                args[3] += cursor[1];
                command = 'S';
            }
            'q' => {
                args[0] += cursor[0];
                args[1] += cursor[1];
                args[2] += cursor[0];
                args[3] += cursor[1];
                command = 'Q';
            }
            't' => {
                args[0] += cursor[0];
                args[1] += cursor[1];
                command = 'T';
            }
            'a' => {
                args[5] += cursor[0];
                args[6] += cursor[1];
                command = 'A';
            }
            _ => {}
        }

        match command {
            'M' => {
                cursor = [args[0], args[1]];
                start = cursor;
            }
            'H' => {
                cursor[0] = args[0];
            }
            'V' => {
                cursor[1] = args[0];
            }
            'L' | 'T' => cursor = [args[0], args[1]],
            'C' => {
                cursor = [args[4], args[5]];
            }
            'S' | 'Q' => cursor = [args[2], args[3]],
            'A' => {
                cursor = [args[5], args[6]];
            }
            'z' | 'Z' => {
                cursor = start;
                command = 'z';
            }
            _ => {}
        }

        new_data.push(PathDataItem { command, args });
    }

    new_data
}

fn gather_points(path_data: &[PathDataItem]) -> Vec<[f64; 2]> {
    let mut points = Vec::<[f64; 2]>::new();
    let mut start = [0.0f64, 0.0];
    let mut cursor = [0.0f64, 0.0];

    for item in path_data {
        match item.command {
            'M' => {
                cursor = [item.args[0], item.args[1]];
                start = cursor;
                points.push(cursor);
            }
            'L' | 'T' => {
                cursor = [item.args[0], item.args[1]];
                points.push(cursor);
            }
            'H' => {
                cursor = [item.args[0], cursor[1]];
                points.push(cursor);
            }
            'V' => {
                cursor = [cursor[0], item.args[0]];
                points.push(cursor);
            }
            'C' => {
                points.push([item.args[0], item.args[1]]);
                points.push([item.args[2], item.args[3]]);
                cursor = [item.args[4], item.args[5]];
                points.push(cursor);
            }
            'S' | 'Q' => {
                points.push([item.args[0], item.args[1]]);
                cursor = [item.args[2], item.args[3]];
                points.push(cursor);
            }
            'A' => {
                cursor = [item.args[5], item.args[6]];
                points.push(cursor);
            }
            'z' => {
                cursor = start;
                points.push(cursor);
            }
            _ => {}
        }
    }

    points
}

struct Bounds {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

fn compute_bounds(points: &[[f64; 2]]) -> Option<Bounds> {
    let first = points.first()?;
    let mut bounds = Bounds {
        min_x: first[0],
        min_y: first[1],
        max_x: first[0],
        max_y: first[1],
    };
    for point in points.iter().skip(1) {
        bounds.min_x = bounds.min_x.min(point[0]);
        bounds.min_y = bounds.min_y.min(point[1]);
        bounds.max_x = bounds.max_x.max(point[0]);
        bounds.max_y = bounds.max_y.max(point[1]);
    }
    Some(bounds)
}
