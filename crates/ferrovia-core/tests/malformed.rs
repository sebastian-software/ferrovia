use std::path::PathBuf;

use ferrovia_core::{Config, optimize};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace member")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn malformed_svg_corpus_returns_parse_errors() {
    let root = workspace_root().join("tests/fixtures/malformed");
    for name in [
        "mismatched-close.svg",
        "truncated-attribute.svg",
        "unquoted-attribute.svg",
        "unterminated-cdata.svg",
        "unterminated-comment.svg",
    ] {
        let svg = std::fs::read_to_string(root.join(name)).expect("fixture");
        let result = optimize(&svg, &Config::default());
        assert!(result.is_err(), "expected parse error for {name}");
    }
}
