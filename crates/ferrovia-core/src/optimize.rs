use crate::config::Config;
use crate::error::Result;
use crate::parser::parse;
use crate::plugins::apply_plugins;
use crate::serializer::serialize;

/// Result of an optimize pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptimizeResult {
    pub data: String,
}

/// Optimize an SVG string with the supplied config.
///
/// # Errors
///
/// Returns an error when the input cannot be parsed or when the config references
/// a plugin that is not implemented by the current ferrovia build.
pub fn optimize(svg: &str, config: &Config) -> Result<OptimizeResult> {
    let mut current = svg.to_string();
    let passes = if config.multipass { 10 } else { 1 };

    for _ in 0..passes {
        let mut doc = parse(&current)?;
        apply_plugins(&mut doc, config)?;
        let next = serialize(&doc, &config.js2svg);
        if next == current {
            current = next;
            break;
        }
        current = next;
    }

    Ok(OptimizeResult { data: current })
}
