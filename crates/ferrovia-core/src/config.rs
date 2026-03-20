use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Serialization options matching the SVGO config shape where possible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Js2Svg {
    pub pretty: bool,
    pub indent: usize,
}

impl Default for Js2Svg {
    fn default() -> Self {
        Self {
            pretty: false,
            indent: 2,
        }
    }
}

/// Top-level optimizer config.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub multipass: bool,
    pub js2svg: Js2Svg,
    pub plugins: Vec<PluginSpec>,
}

/// Plugin specification compatible with SVGO's config model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PluginSpec {
    Name(String),
    Configured(PluginConfig),
}

impl PluginSpec {
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Name(name) => name,
            Self::Configured(config) => &config.name,
        }
    }

    #[must_use]
    pub fn params(&self) -> Option<&Value> {
        match self {
            Self::Name(_) => None,
            Self::Configured(config) => config.params.as_ref(),
        }
    }

    #[must_use]
    pub fn enabled(&self) -> bool {
        match self {
            Self::Name(_) => true,
            Self::Configured(config) => config.enabled,
        }
    }
}

/// Structured plugin config object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PluginConfig {
    pub name: String,
    pub params: Option<Value>,
    pub enabled: bool,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            params: None,
            enabled: true,
        }
    }
}
