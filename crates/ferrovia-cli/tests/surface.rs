use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace member")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn cli_accepts_string_input() {
    let output = Command::new(env!("CARGO_BIN_EXE_ferrovia-cli"))
        .args([
            "--string",
            r#"<svg xmlns="http://www.w3.org/2000/svg"><title>Hello</title><g/></svg>"#,
            "--config",
        ])
        .arg(workspace_root().join("tests/fixtures/oracle/remove-comments.config.json"))
        .output()
        .expect("run cli");

    assert!(output.status.success(), "{output:?}");
}

#[test]
fn cli_accepts_stdin_and_stdout() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ferrovia-cli"))
        .arg("-")
        .arg(workspace_root().join("tests/fixtures/oracle/remove-comments.config.json"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn cli");

    std::io::Write::write_all(
        child.stdin.as_mut().expect("stdin"),
        br#"<?xml version="1.0"?><!DOCTYPE svg><svg xmlns="http://www.w3.org/2000/svg"><!--remove me--><metadata>meta</metadata><title>Hello</title><desc>World</desc><g><text>Hi</text></g></svg>"#,
    )
    .expect("write stdin");

    let output = child.wait_with_output().expect("wait");
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).expect("utf8"),
        r#"<svg xmlns="http://www.w3.org/2000/svg"><desc>World</desc><g><text>Hi</text></g></svg>"#
    );
}

#[test]
fn cli_writes_to_output_file() {
    let root = workspace_root();
    let temp_dir = std::env::temp_dir().join(format!("ferrovia-cli-{}", std::process::id()));
    fs::create_dir_all(&temp_dir).expect("temp dir");
    let output_path = temp_dir.join("out.svg");

    let output = Command::new(env!("CARGO_BIN_EXE_ferrovia-cli"))
        .arg(root.join("tests/fixtures/oracle/remove-comments.svg"))
        .arg(root.join("tests/fixtures/oracle/remove-comments.config.json"))
        .arg("-o")
        .arg(&output_path)
        .output()
        .expect("run cli");

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        fs::read_to_string(&output_path).expect("read output"),
        r#"<svg xmlns="http://www.w3.org/2000/svg"><!--!keep legal--><desc>World</desc><g><text>Hi</text></g></svg>"#
    );
}
