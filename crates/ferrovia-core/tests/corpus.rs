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
fn optional_svg_corpus_matches_svgo_preset_default() {
    let root = workspace_root();
    let corpus_dir = std::env::var_os("FERROVIA_SVGO_TEST_SUITE_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            let candidate = root.join("vendor/svgo-test-suite");
            candidate.exists().then_some(candidate)
        });

    let Some(corpus_dir) = corpus_dir else {
        eprintln!("skipping optional corpus test because FERROVIA_SVGO_TEST_SUITE_DIR is not set");
        return;
    };
    if !corpus_dir.exists() {
        eprintln!("skipping optional corpus test because corpus dir does not exist: {}", corpus_dir.display());
        return;
    }

    let limit = std::env::var("FERROVIA_SVGO_TEST_SUITE_LIMIT")
        .ok()
        .and_then(|value| value.parse::<usize>().ok());
    let mut fixtures = Vec::new();
    collect_svg_files(corpus_dir.as_path(), &mut fixtures);
    fixtures.sort();
    if let Some(limit) = limit {
        fixtures.truncate(limit);
    }
    if fixtures.is_empty() {
        eprintln!("skipping optional corpus test because no .svg files were found in {}", corpus_dir.display());
        return;
    }

    let config = Config {
        plugins: vec![PluginSpec::Name("preset-default".to_string())],
        ..Config::default()
    };

    let mut mismatches = Vec::new();
    for svg_path in &fixtures {
        let svg = match fs::read_to_string(svg_path) {
            Ok(svg) => svg,
            Err(error) => {
                mismatches.push(format!("{}: failed to read fixture: {error}", svg_path.display()));
                continue;
            }
        };
        let actual = match optimize(&svg, &config) {
            Ok(result) => normalize(&result.data),
            Err(error) => {
                mismatches.push(format!("{}: ferrovia optimize failed: {error}", svg_path.display()));
                continue;
            }
        };
        let oracle = match run_oracle(svg_path) {
            Ok(result) => result,
            Err(error) => {
                mismatches.push(format!("{}: oracle failed: {error}", svg_path.display()));
                continue;
            }
        };
        if actual != oracle {
            mismatches.push(svg_path.display().to_string());
        }
    }

    assert!(
        mismatches.is_empty(),
        "corpus mismatches: {} / {} files\n{}",
        mismatches.len(),
        fixtures.len(),
        mismatches
            .into_iter()
            .take(20)
            .collect::<Vec<_>>()
            .join("\n")
    );
}

fn collect_svg_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect_svg_files(path.as_path(), files);
        } else if path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("svg")) {
            files.push(path);
        }
    }
}

fn run_oracle(svg_path: &Path) -> std::io::Result<String> {
    let root = workspace_root();
    let output = Command::new("node")
        .arg(root.join("scripts/run-svgo-oracle.mjs"))
        .arg(svg_path)
        .arg(root.join("tests/fixtures/preset-default/default.config.json"))
        .current_dir(&root)
        .output()?;
    assert!(output.status.success(), "oracle failed: {output:?}");
    Ok(normalize(&String::from_utf8(output.stdout).expect("utf8")))
}

fn normalize(value: &str) -> String {
    value.trim().replace("\r\n", "\n")
}
