use crate::svgo::tools::{remove_leading_zero, to_fixed};
use crate::types::PathDataItem;

const ARGS_COUNT_PER_COMMAND: &[(char, usize)] = &[
    ('M', 2),
    ('m', 2),
    ('Z', 0),
    ('z', 0),
    ('L', 2),
    ('l', 2),
    ('H', 1),
    ('h', 1),
    ('V', 1),
    ('v', 1),
    ('C', 6),
    ('c', 6),
    ('S', 4),
    ('s', 4),
    ('Q', 4),
    ('q', 4),
    ('T', 2),
    ('t', 2),
    ('A', 7),
    ('a', 7),
];

#[must_use]
pub fn parse_path_data(string: &str) -> Vec<PathDataItem> {
    let mut path_data = Vec::new();
    let mut command = None;
    let mut args = Vec::<f64>::new();
    let mut chars = string.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch.is_ascii_whitespace() || ch == ',' {
            continue;
        }

        if is_command(ch) {
            command = Some(ch);
            if args_count_for_command(ch) == 0 {
                path_data.push(PathDataItem {
                    command: ch,
                    args: Vec::new(),
                });
            } else {
                args.clear();
            }
            continue;
        }

        let Some(current_command) = command else {
            return path_data;
        };

        let mut number = String::from(ch);
        while let Some(next) = chars.peek().copied() {
            if next.is_ascii_whitespace() || next == ',' {
                chars.next();
                break;
            }
            if is_command(next) {
                break;
            }
            if matches!(next, '-' | '+') && !matches!(number.chars().last(), Some('e' | 'E')) {
                break;
            }
            number.push(next);
            chars.next();
        }

        let Ok(parsed) = number.parse::<f64>() else {
            return path_data;
        };
        args.push(parsed);
        if args.len() == args_count_for_command(current_command) {
            path_data.push(PathDataItem {
                command: current_command,
                args: args.clone(),
            });
            if current_command == 'M' {
                command = Some('L');
            } else if current_command == 'm' {
                command = Some('l');
            }
            args.clear();
        }
    }

    path_data
}

#[must_use]
pub fn stringify_path_data(path_data: &[PathDataItem], precision: Option<usize>) -> String {
    let mut out = String::new();
    for item in path_data {
        out.push(item.command);
        if !item.args.is_empty() {
            let mut first = true;
            for value in &item.args {
                if !first {
                    out.push(' ');
                }
                first = false;
                let serialized = precision.map_or_else(
                    || remove_leading_zero(*value),
                    |precision| remove_leading_zero(to_fixed(*value, precision)),
                );
                out.push_str(serialized.as_str());
            }
        }
    }
    out
}

#[must_use]
pub const fn is_command(value: char) -> bool {
    let mut index = 0usize;
    while index < ARGS_COUNT_PER_COMMAND.len() {
        if ARGS_COUNT_PER_COMMAND[index].0 == value {
            return true;
        }
        index += 1;
    }
    false
}

#[must_use]
pub const fn args_count_for_command(command: char) -> usize {
    let mut index = 0usize;
    while index < ARGS_COUNT_PER_COMMAND.len() {
        if ARGS_COUNT_PER_COMMAND[index].0 == command {
            return ARGS_COUNT_PER_COMMAND[index].1;
        }
        index += 1;
    }
    0
}
