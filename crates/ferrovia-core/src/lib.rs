//! Core optimizer primitives for ferrovia.

pub mod ast;
pub mod config;
pub mod error;
pub mod optimize;
pub mod parser;
pub mod plugins;
pub mod serializer;

pub use config::{Config, Js2Svg, PluginConfig, PluginSpec};
pub use error::{FerroviaError, Result};
pub use optimize::{OptimizeResult, optimize};
