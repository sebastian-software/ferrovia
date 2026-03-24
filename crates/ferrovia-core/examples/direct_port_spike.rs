use std::fs;
use std::path::PathBuf;

use ferrovia_core::svgo_spike::{
    DirectPortSpikeConfig, SpikePluginFamily, optimize_with_direct_port_spike,
};
use ferrovia_core::{Config, Result};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let input = args
        .next()
        .map(PathBuf::from)
        .expect("usage: cargo run -p ferrovia-core --example direct_port_spike -- <input.svg> <config.json> [families]");
    let config = args
        .next()
        .map(PathBuf::from)
        .expect("missing config path");
    let families = args.next().unwrap_or_else(|| "cleanupIds,convertPathData,inlineStyles".to_string());

    let svg = fs::read_to_string(input).expect("read input svg");
    let config: Config = serde_json::from_str(&fs::read_to_string(config).expect("read config"))
        .expect("parse config json");
    let spike = DirectPortSpikeConfig {
        families: parse_families(families.as_str()),
    };
    let result = optimize_with_direct_port_spike(&svg, &config, &spike)?;
    print!("{}", result.data);
    Ok(())
}

fn parse_families(value: &str) -> Vec<SpikePluginFamily> {
    let mut families = Vec::new();
    for item in value.split(',').map(str::trim).filter(|item| !item.is_empty()) {
        match item {
            "cleanupIds" => families.push(SpikePluginFamily::CleanupIds),
            "convertPathData" => families.push(SpikePluginFamily::ConvertPathData),
            "inlineStyles" => families.push(SpikePluginFamily::InlineStyles),
            other => panic!("unknown spike family: {other}"),
        }
    }
    families
}
