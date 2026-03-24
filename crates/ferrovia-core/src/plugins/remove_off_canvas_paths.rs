use crate::path::parse_path_data;
use crate::plugins::_path::intersects;
use crate::types::{PathDataItem, XastChild, XastRoot};

#[derive(Clone, Copy)]
struct ViewBoxData {
    top: f64,
    right: f64,
    bottom: f64,
    left: f64,
    width: f64,
    height: f64,
}

/// Apply the `removeOffCanvasPaths` plugin.
///
/// # Errors
///
/// This direct port currently does not return plugin-specific runtime errors.
pub fn apply(root: &mut XastRoot) -> crate::error::Result<()> {
    let mut view_box_data = None::<ViewBoxData>;
    remove_off_canvas_paths(&mut root.children, true, &mut view_box_data);
    Ok(())
}

fn remove_off_canvas_paths(
    children: &mut Vec<XastChild>,
    parent_is_root: bool,
    view_box_data: &mut Option<ViewBoxData>,
) {
    let mut index = 0usize;
    while index < children.len() {
        let mut removed = false;
        let mut skip_children = false;
        let mut should_remove_path = false;
        if let XastChild::Element(element) = &mut children[index] {
            if element.name == "svg" && parent_is_root {
                *view_box_data = parse_view_box(element);
            }

            if element.get_attribute("transform").is_some() {
                skip_children = true;
            } else {
                if element.name == "path"
                    && let (Some(d), Some(view_box)) = (element.get_attribute("d"), *view_box_data)
                {
                    let mut path_data = parse_path_data(d);
                    let visible = path_data.iter().any(|item| {
                        item.command == 'M'
                            && item.args.len() >= 2
                            && item.args[0] >= view_box.left
                            && item.args[0] <= view_box.right
                            && item.args[1] >= view_box.top
                            && item.args[1] <= view_box.bottom
                    });
                    if !visible {
                        if path_data.len() == 2 {
                            path_data.push(PathDataItem {
                                command: 'z',
                                args: Vec::new(),
                            });
                        }
                        let view_box_path = vec![
                            PathDataItem {
                                command: 'M',
                                args: vec![view_box.left, view_box.top],
                            },
                            PathDataItem {
                                command: 'h',
                                args: vec![view_box.width],
                            },
                            PathDataItem {
                                command: 'v',
                                args: vec![view_box.height],
                            },
                            PathDataItem {
                                command: 'H',
                                args: vec![view_box.left],
                            },
                            PathDataItem {
                                command: 'z',
                                args: Vec::new(),
                            },
                        ];
                        should_remove_path = !intersects(&view_box_path, &path_data);
                    }
                }

                if !should_remove_path {
                    remove_off_canvas_paths(&mut element.children, false, view_box_data);
                }
            }
        }
        if should_remove_path {
            children.remove(index);
            removed = true;
        }
        if !removed {
            if skip_children {
                index += 1;
                continue;
            }
            index += 1;
        }
    }
}

fn parse_view_box(element: &crate::types::XastElement) -> Option<ViewBoxData> {
    let mut view_box = element.get_attribute("viewBox").map_or_else(
        || {
            if let (Some(width), Some(height)) = (
                element.get_attribute("width"),
                element.get_attribute("height"),
            ) {
                format!("0 0 {width} {height}")
            } else {
                String::new()
            }
        },
        str::to_string,
    );

    view_box = view_box
        .replace([',', '+'], " ")
        .replace("px", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    let numbers = view_box.split(' ').collect::<Vec<_>>();
    if numbers.len() != 4 {
        return None;
    }

    let left = numbers[0].parse::<f64>().ok()?;
    let top = numbers[1].parse::<f64>().ok()?;
    let width = numbers[2].parse::<f64>().ok()?;
    let height = numbers[3].parse::<f64>().ok()?;

    Some(ViewBoxData {
        left,
        top,
        right: left + width,
        bottom: top + height,
        width,
        height,
    })
}
