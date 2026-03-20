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

#[test]
fn matches_svgo_oracle_for_remove_empty_containers() {
    assert_oracle_fixture("remove-empty-containers");
}

#[test]
fn matches_svgo_oracle_for_move_group_attrs_to_elems() {
    assert_oracle_fixture("move-group-attrs-to-elems");
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

    assert!(expected.status.success(), "oracle failed: {expected:?}");

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
    assert_eq!(result.data, r"<svg><!--keep me--></svg>");
}

#[test]
fn sort_attrs_supports_alphabetical_xmlns_order() {
    let svg = r#"<svg foo="bar" xmlns="http://www.w3.org/2000/svg" height="10" baz="quux" width="10" hello="world"><rect x="0" y="0" width="100" height="100" stroke-width="1" stroke-linejoin="round" fill="red" stroke="orange" xmlns="http://www.w3.org/2000/svg"/></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Configured(PluginConfig {
            name: "sortAttrs".to_string(),
            params: Some(json!({ "xmlnsOrder": "alphabetical" })),
            enabled: true,
        })],
        js2svg: ferrovia_core::Js2Svg {
            pretty: true,
            indent: 4,
        },
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data.trim(),
        r#"<svg width="10" height="10" baz="quux" foo="bar" hello="world" xmlns="http://www.w3.org/2000/svg">
    <rect width="100" height="100" x="0" y="0" fill="red" stroke="orange" stroke-linejoin="round" stroke-width="1" xmlns="http://www.w3.org/2000/svg"/>
</svg>"#
    );
}

#[test]
fn remove_empty_containers_removes_empty_defs_and_referencing_use() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><defs id="gone"/><use href="#gone"/><mask id="keep"/></svg>"##;
    let config = Config {
        plugins: vec![PluginSpec::Name("removeEmptyContainers".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><mask id="keep"/></svg>"#
    );
}

#[test]
fn remove_empty_containers_preserves_switch_child_and_filtered_group() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><style>g.keep{filter:url(#fx)}</style><switch><g/></switch><g class="keep"/><g/></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Name("removeEmptyContainers".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><style>g.keep{filter:url(#fx)}</style><switch><g/></switch><g class="keep"/></svg>"#
    );
}

#[test]
fn move_group_attrs_to_elems_keeps_group_transform_when_url_reference_is_present() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><g transform="scale(2)" clip-path="url(#clip)"><path d="M0 0"/></g></svg>"#;
    let config = Config {
        plugins: vec![PluginSpec::Name("moveGroupAttrsToElems".to_string())],
        ..Config::default()
    };

    let result = optimize(svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><g transform="scale(2)" clip-path="url(#clip)"><path d="M0 0"/></g></svg>"#
    );
}
