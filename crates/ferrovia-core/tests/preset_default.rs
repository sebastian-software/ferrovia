use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use ferrovia_core::{Config, PluginSpec, optimize};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace member")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn preset_default_fixture_matrix_matches_svgo() {
    for fixture_path in discover_preset_fixtures() {
        assert_preset_fixture(&fixture_path);
    }
}

fn discover_preset_fixtures() -> Vec<PathBuf> {
    let fixture_dir = workspace_root().join("tests/fixtures/preset-default");
    let mut fixtures = fs::read_dir(&fixture_dir)
        .expect("preset fixture dir")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "svg"))
        .collect::<Vec<_>>();
    fixtures.sort();
    fixtures
}

fn assert_preset_fixture(svg_path: &Path) {
    let root = workspace_root();
    let config_path = svg_path.with_extension("config.json");
    let config = if config_path.exists() {
        serde_json::from_str::<Config>(&fs::read_to_string(&config_path).expect("config"))
            .expect("parse config")
    } else {
        Config {
            plugins: vec![PluginSpec::Name("preset-default".to_string())],
            ..Config::default()
        }
    };

    let actual = optimize(&fs::read_to_string(svg_path).expect("svg"), &config).expect("optimize");
    let oracle = Command::new("node")
        .arg(root.join("scripts/run-svgo-oracle.mjs"))
        .arg(svg_path)
        .args(config_path.exists().then_some(config_path.as_path()))
        .current_dir(&root)
        .output()
        .expect("run oracle");

    assert!(oracle.status.success(), "oracle failed: {oracle:?}");
    assert_eq!(
        normalize(&actual.data),
        normalize(&String::from_utf8(oracle.stdout).expect("utf8")),
        "{}",
        svg_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("preset fixture")
    );
}

fn normalize(value: &str) -> String {
    value.trim().replace("\r\n", "\n")
}
