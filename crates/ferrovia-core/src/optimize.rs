use crate::builtin::expand_plugins;
use crate::config::Config;
use crate::error::Result;
use crate::parser::parse_svg;
use crate::plugins::apply_plugin;
use crate::stringifier::stringify_svg;

/// Result of an optimize pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptimizeResult {
    pub data: String,
}

/// Optimize an SVG string with the supplied config.
///
/// # Errors
///
/// Returns an error when the input cannot be parsed or when the config
/// references a plugin that is not implemented by the current rewrite stage.
pub fn optimize(svg: &str, config: &Config) -> Result<OptimizeResult> {
    let mut current = svg.to_string();
    let passes = if config.multipass { 10 } else { 1 };

    for _ in 0..passes {
        let mut root = parse_svg(&current, None)?;
        for plugin in expand_plugins(config) {
            apply_plugin(&mut root, &plugin)?;
        }
        let next = stringify_svg(&root, Some(config.js2svg.clone()));
        if next == current {
            current = next;
            break;
        }
        current = next;
    }

    Ok(OptimizeResult { data: current })
}
