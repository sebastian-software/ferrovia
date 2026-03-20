use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace member")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn cli_matches_supported_fixture() {
    let root = workspace_root();
    let output = Command::new(env!("CARGO_BIN_EXE_ferrovia-cli"))
        .arg(root.join("tests/fixtures/oracle/remove-comments.svg"))
        .arg(root.join("tests/fixtures/oracle/remove-comments.config.json"))
        .output()
        .expect("run cli");

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).expect("utf8"),
        r#"<svg xmlns="http://www.w3.org/2000/svg"><!--!keep legal--><desc>World</desc><g><text>Hi</text></g></svg>"#
    );
}
