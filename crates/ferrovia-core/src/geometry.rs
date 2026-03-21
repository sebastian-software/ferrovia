#![allow(dead_code)]

use svgtypes::{PathParser, PathSegment, TransformListParser, TransformListToken};

/// Internal ferrovia representation for parsed path commands.
#[derive(Debug, Clone, PartialEq)]
pub enum PathCommand {
    MoveTo {
        abs: bool,
        x: f64,
        y: f64,
    },
    LineTo {
        abs: bool,
        x: f64,
        y: f64,
    },
    HorizontalLineTo {
        abs: bool,
        x: f64,
    },
    VerticalLineTo {
        abs: bool,
        y: f64,
    },
    CurveTo {
        abs: bool,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        x: f64,
        y: f64,
    },
    SmoothCurveTo {
        abs: bool,
        x2: f64,
        y2: f64,
        x: f64,
        y: f64,
    },
    Quadratic {
        abs: bool,
        x1: f64,
        y1: f64,
        x: f64,
        y: f64,
    },
    SmoothQuadratic {
        abs: bool,
        x: f64,
        y: f64,
    },
    EllipticalArc {
        abs: bool,
        rx: f64,
        ry: f64,
        x_axis_rotation: f64,
        large_arc: bool,
        sweep: bool,
        x: f64,
        y: f64,
    },
    ClosePath {
        abs: bool,
    },
}

/// Internal ferrovia representation for parsed transforms.
#[derive(Debug, Clone, PartialEq)]
pub enum TransformOperation {
    Matrix {
        a: f64,
        b: f64,
        c: f64,
        d: f64,
        e: f64,
        f: f64,
    },
    Translate {
        tx: f64,
        ty: f64,
    },
    Scale {
        sx: f64,
        sy: f64,
    },
    Rotate {
        angle: f64,
    },
    SkewX {
        angle: f64,
    },
    SkewY {
        angle: f64,
    },
}

pub(crate) fn parse_path_commands(input: &str) -> std::result::Result<Vec<PathCommand>, String> {
    PathParser::from(input)
        .map(|segment| segment.map(PathCommand::from).map_err(|error| error.to_string()))
        .collect()
}

pub(crate) fn parse_transform_operations(
    input: &str,
) -> std::result::Result<Vec<TransformOperation>, String> {
    TransformListParser::from(input)
        .map(|token| {
            token
                .map(TransformOperation::from)
                .map_err(|error| error.to_string())
        })
        .collect()
}

impl From<PathSegment> for PathCommand {
    fn from(segment: PathSegment) -> Self {
        match segment {
            PathSegment::MoveTo { abs, x, y } => Self::MoveTo { abs, x, y },
            PathSegment::LineTo { abs, x, y } => Self::LineTo { abs, x, y },
            PathSegment::HorizontalLineTo { abs, x } => Self::HorizontalLineTo { abs, x },
            PathSegment::VerticalLineTo { abs, y } => Self::VerticalLineTo { abs, y },
            PathSegment::CurveTo {
                abs,
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => Self::CurveTo {
                abs,
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            },
            PathSegment::SmoothCurveTo { abs, x2, y2, x, y } => {
                Self::SmoothCurveTo { abs, x2, y2, x, y }
            }
            PathSegment::Quadratic { abs, x1, y1, x, y } => Self::Quadratic { abs, x1, y1, x, y },
            PathSegment::SmoothQuadratic { abs, x, y } => Self::SmoothQuadratic { abs, x, y },
            PathSegment::EllipticalArc {
                abs,
                rx,
                ry,
                x_axis_rotation,
                large_arc,
                sweep,
                x,
                y,
            } => Self::EllipticalArc {
                abs,
                rx,
                ry,
                x_axis_rotation,
                large_arc,
                sweep,
                x,
                y,
            },
            PathSegment::ClosePath { abs } => Self::ClosePath { abs },
        }
    }
}

impl From<TransformListToken> for TransformOperation {
    fn from(token: TransformListToken) -> Self {
        match token {
            TransformListToken::Matrix { a, b, c, d, e, f } => Self::Matrix { a, b, c, d, e, f },
            TransformListToken::Translate { tx, ty } => Self::Translate { tx, ty },
            TransformListToken::Scale { sx, sy } => Self::Scale { sx, sy },
            TransformListToken::Rotate { angle } => Self::Rotate { angle },
            TransformListToken::SkewX { angle } => Self::SkewX { angle },
            TransformListToken::SkewY { angle } => Self::SkewY { angle },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PathCommand, TransformOperation, parse_path_commands, parse_transform_operations};

    #[test]
    fn parses_svgtypes_path_commands_into_internal_ir() {
        let commands = parse_path_commands("M0 0L10 10h5v-2a4 5 0 1 1 20 30z").expect("path");
        assert_eq!(
            commands,
            vec![
                PathCommand::MoveTo {
                    abs: true,
                    x: 0.0,
                    y: 0.0,
                },
                PathCommand::LineTo {
                    abs: true,
                    x: 10.0,
                    y: 10.0,
                },
                PathCommand::HorizontalLineTo { abs: false, x: 5.0 },
                PathCommand::VerticalLineTo { abs: false, y: -2.0 },
                PathCommand::EllipticalArc {
                    abs: false,
                    rx: 4.0,
                    ry: 5.0,
                    x_axis_rotation: 0.0,
                    large_arc: true,
                    sweep: true,
                    x: 20.0,
                    y: 30.0,
                },
                PathCommand::ClosePath { abs: false },
            ]
        );
    }

    #[test]
    fn parses_rotate_about_center_into_expanded_transform_ir() {
        let operations =
            parse_transform_operations("translate(4,5) rotate(90 10 20)").expect("transform");
        assert_eq!(
            operations,
            vec![
                TransformOperation::Translate { tx: 4.0, ty: 5.0 },
                TransformOperation::Translate { tx: 10.0, ty: 20.0 },
                TransformOperation::Rotate { angle: 90.0 },
                TransformOperation::Translate {
                    tx: -10.0,
                    ty: -20.0,
                },
            ]
        );
    }

    #[test]
    fn rejects_invalid_path_data() {
        let error = parse_path_commands("M 0").expect_err("invalid path must fail");
        assert!(!error.is_empty());
    }
}
