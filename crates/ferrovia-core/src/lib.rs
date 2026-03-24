//! Core optimizer primitives for ferrovia.

pub mod builtin;
pub mod config;
pub mod error;
pub mod optimize;
pub mod parser;
pub mod plugins;
pub mod stringifier;
pub mod svgo;
pub mod types;
pub mod util;
pub mod xast;

pub use config::{Config, Js2Svg, PluginConfig, PluginSpec};
pub use error::{FerroviaError, Result};
pub use optimize::{OptimizeResult, optimize};
