use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use ferrovia_core::{Config, Js2Svg, PluginConfig, PluginSpec, optimize};
use serde_json::Value;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace member")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn upstream_plugin_fixture_matrix_matches_svgo() {
    let fixtures = [
        ("cleanupAttrs", "cleanupAttrs.01.svg.txt"),
        ("cleanupAttrs", "cleanupAttrs.02.svg.txt"),
        ("removeDesc", "removeDesc.01.svg.txt"),
        ("removeEditorsNSData", "removeEditorsNSData.01.svg.txt"),
        ("removeEditorsNSData", "removeEditorsNSData.02.svg.txt"),
        ("removeEmptyAttrs", "removeEmptyAttrs.01.svg.txt"),
        ("removeEmptyAttrs", "removeEmptyAttrs.02.svg.txt"),
        ("removeEmptyText", "removeEmptyText.01.svg.txt"),
        ("removeEmptyText", "removeEmptyText.02.svg.txt"),
        ("removeEmptyText", "removeEmptyText.03.svg.txt"),
        ("sortAttrs", "sortAttrs.01.svg.txt"),
        ("sortAttrs", "sortAttrs.02.svg.txt"),
        ("sortAttrs", "sortAttrs.03.svg.txt"),
        ("sortAttrs", "sortAttrs.04.svg.txt"),
        ("removeTitle", "removeTitle.01.svg.txt"),
    ];

    for (plugin, file_name) in fixtures {
        assert_plugin_fixture(plugin, file_name);
    }
}

fn assert_plugin_fixture(plugin: &str, file_name: &str) {
    let fixture_path = workspace_root()
        .join("tests/upstream/svgo-v4.0.1/plugins")
        .join(file_name);
    let fixture = SvgTxtFixture::load(&fixture_path);

    let config = Config {
        js2svg: Js2Svg {
            pretty: true,
            indent: 4,
        },
        plugins: vec![PluginSpec::Configured(PluginConfig {
            name: plugin.to_string(),
            params: fixture.params.clone(),
            enabled: true,
        })],
        ..Config::default()
    };

    let actual = optimize(&fixture.original, &config).expect("optimize");
    let actual_normalized = normalize(&actual.data);
    assert_eq!(actual_normalized, fixture.expected, "{plugin} {file_name}");

    let oracle = run_oracle(plugin, &fixture).expect("oracle");
    assert_eq!(actual_normalized, oracle, "{plugin} {file_name}");
}

fn run_oracle(plugin: &str, fixture: &SvgTxtFixture) -> std::io::Result<String> {
    let root = workspace_root();
    let temp_base = std::env::temp_dir().join(format!(
        "ferrovia-fixture-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ));
    fs::create_dir_all(&temp_base)?;
    let svg_path = temp_base.join("fixture.svg");
    let config_path = temp_base.join("fixture.config.json");
    fs::write(&svg_path, &fixture.original)?;
    fs::write(
        &config_path,
        serde_json::to_vec(&serde_json::json!({
            "plugins": [
                {
                    "name": plugin,
                    "params": fixture
                        .params
                        .clone()
                        .unwrap_or_else(|| Value::Object(serde_json::Map::default())),
                }
            ],
            "js2svg": {
                "pretty": true,
                "indent": 4
            }
        }))
        .expect("config"),
    )?;

    let output = Command::new("node")
        .arg(root.join("scripts/run-svgo-oracle.mjs"))
        .arg(&svg_path)
        .arg(&config_path)
        .current_dir(&root)
        .output()?;

    let _ = fs::remove_file(&svg_path);
    let _ = fs::remove_file(&config_path);
    let _ = fs::remove_dir(&temp_base);

    assert!(output.status.success(), "oracle failed: {output:?}");
    Ok(normalize(&String::from_utf8(output.stdout).expect("utf8")))
}

#[derive(Debug)]
struct SvgTxtFixture {
    original: String,
    expected: String,
    params: Option<Value>,
}

impl SvgTxtFixture {
    fn load(path: &Path) -> Self {
        let raw = normalize(&fs::read_to_string(path).expect("fixture"));
        let sections = split_sections(&raw);
        let test_sections = if sections.iter().any(|(separator, _)| separator == "===") {
            &sections[1..]
        } else {
            &sections[..]
        };

        let mut payload = Vec::new();
        for (separator, body) in test_sections {
            if separator == "@@@" || payload.is_empty() {
                payload.push(body.clone());
            }
        }

        let original = payload.first().cloned().expect("original");
        let expected = payload.get(1).cloned().expect("expected");
        let params = payload
            .get(2)
            .map(|raw| serde_json::from_str::<Value>(raw).expect("params json"));

        Self {
            original,
            expected,
            params,
        }
    }
}

fn split_sections(input: &str) -> Vec<(String, String)> {
    let mut sections = Vec::new();
    let mut current = Vec::new();
    let mut current_separator = String::new();
    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed == "===" || trimmed == "@@@" {
            sections.push((
                current_separator.clone(),
                current.join("\n").trim().to_string(),
            ));
            current_separator = trimmed.to_string();
            current.clear();
        } else {
            current.push(line.to_string());
        }
    }
    sections.push((current_separator, current.join("\n").trim().to_string()));
    sections
}

fn normalize(value: &str) -> String {
    value.trim().replace("\r\n", "\n")
}
