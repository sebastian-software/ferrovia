use std::path::PathBuf;
use std::process::Command;

use ferrovia_core::{Config, PluginConfig, PluginSpec, optimize};
use serde_json::json;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace member")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn roundtrip_simple_svg_is_stable() {
    let svg = std::fs::read_to_string(workspace_root().join("tests/fixtures/roundtrip/simple.svg"))
        .expect("fixture");
    let result = optimize(&svg, &Config::default()).expect("optimize");
    assert_eq!(result.data, svg.trim().to_string());
}

#[test]
fn removes_supported_structural_nodes() {
    let svg =
        std::fs::read_to_string(workspace_root().join("tests/fixtures/oracle/remove-comments.svg"))
            .expect("fixture");

    let config = Config {
        plugins: vec![
            PluginSpec::Name("removeXMLProcInst".to_string()),
            PluginSpec::Name("removeDoctype".to_string()),
            PluginSpec::Configured(PluginConfig {
                name: "removeComments".to_string(),
                params: Some(json!({ "preservePatterns": ["^!"] })),
                enabled: true,
            }),
            PluginSpec::Name("removeMetadata".to_string()),
            PluginSpec::Name("removeTitle".to_string()),
        ],
        ..Config::default()
    };

    let result = optimize(&svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><!--!keep legal--><desc>World</desc><g><text>Hi</text></g></svg>"#
    );
}

#[test]
fn matches_svgo_oracle_for_supported_fixture() {
    assert_oracle_fixture("remove-comments");
}

#[test]
fn matches_svgo_oracle_for_remove_desc_empty() {
    assert_oracle_fixture("remove-desc-empty");
}

#[test]
fn matches_svgo_oracle_for_remove_dimensions() {
    assert_oracle_fixture("remove-dimensions");
}

#[test]
fn matches_svgo_oracle_for_remove_xmlns() {
    assert_oracle_fixture("remove-xmlns");
}

fn assert_oracle_fixture(name: &str) {
    let root = workspace_root();
    let svg_path = root.join(format!("tests/fixtures/oracle/{name}.svg"));
    let config_path = root.join(format!("tests/fixtures/oracle/{name}.config.json"));
    let node_modules = root.join("node_modules/svgo");

    if !node_modules.exists() {
        eprintln!("skipping oracle test because svgo is not installed");
        return;
    }

    let expected = Command::new("node")
        .arg(root.join("scripts/run-svgo-oracle.mjs"))
        .arg(&svg_path)
        .arg(&config_path)
        .current_dir(&root)
        .output()
        .expect("run oracle");

    assert!(expected.status.success(), "oracle failed: {:?}", expected);

    let config =
        serde_json::from_str::<Config>(&std::fs::read_to_string(config_path).expect("config"))
            .expect("parse config");
    let actual =
        optimize(&std::fs::read_to_string(svg_path).expect("svg"), &config).expect("optimize");

    assert_eq!(
        actual.data,
        String::from_utf8(expected.stdout).expect("utf8")
    );
}

#[test]
fn preset_default_honors_boolean_overrides() {
    let svg =
        r#"<?xml version="1.0"?><!DOCTYPE svg><svg><!--keep me--><metadata>meta</metadata></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Configured(PluginConfig {
            name: "preset-default".to_string(),
            params: Some(json!({
                "overrides": {
                    "removeComments": false
                }
            })),
            enabled: true,
        })],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(result.data, r#"<svg><!--keep me--></svg>"#);
}
