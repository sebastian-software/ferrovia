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
fn optimize_matches_remove_comments_fixture() {
    let root = workspace_root();
    let svg = std::fs::read_to_string(root.join("tests/fixtures/oracle/remove-comments.svg"))
        .expect("read fixture");
    let config = serde_json::from_str::<Config>(
        &std::fs::read_to_string(root.join("tests/fixtures/oracle/remove-comments.config.json"))
            .expect("read config"),
    )
    .expect("parse config");

    let result = optimize(&svg, &config).expect("optimize");
    assert_eq!(
        result.data,
        r#"<svg xmlns="http://www.w3.org/2000/svg"><!--!keep legal--><desc>World</desc><g><text>Hi</text></g></svg>"#
    );
}

#[test]
fn optimize_without_plugins_roundtrips_basic_svg() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><g><text>Hi</text></g></svg>"#;
    let result = optimize(svg, &Config::default()).expect("optimize");
    assert_eq!(result.data, svg);
}
