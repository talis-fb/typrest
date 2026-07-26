use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::tempdir;

fn write_project(root: &Path, main: &str) {
    fs::create_dir_all(root.join("templates/invoice")).unwrap();
    fs::write(
        root.join("papelon.toml"),
        "schema_version = \"v1\"\ntemplates_root = \"templates\"\n",
    )
    .unwrap();
    fs::write(root.join("templates/invoice/main.typ"), main).unwrap();
    fs::write(
        root.join("templates/invoice/schema.json"),
        r#"{"schemaVersion":"v1","request":{"body":{"type":"object"}}}"#,
    )
    .unwrap();
}

#[test]
fn check_succeeds_without_starting_a_server() {
    let root = tempdir().unwrap();
    write_project(root.path(), "Hello");

    let output = Command::new(env!("CARGO_BIN_EXE_papelon"))
        .current_dir(root.path())
        .args(["check", "--color", "never"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("templates compiled successfully"),
        "stderr was: {stderr}"
    );
    assert!(!stderr.contains("server listening"));
}

#[test]
fn check_pretty_prints_annotated_typst_source() {
    let root = tempdir().unwrap();
    write_project(root.path(), "#let broken =");

    let output = Command::new(env!("CARGO_BIN_EXE_papelon"))
        .current_dir(root.path())
        .args(["check", "--color", "never"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error:"));
    assert!(stderr.contains("templates/invoice/main.typ"));
    assert!(stderr.contains("#let broken ="));
    assert!(stderr.contains('^'));
}

#[test]
fn check_json_is_jsonl_and_omits_source_excerpts() {
    let root = tempdir().unwrap();
    write_project(root.path(), "#let broken =");

    let output = Command::new(env!("CARGO_BIN_EXE_papelon"))
        .current_dir(root.path())
        .args(["check", "--log-format", "json"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    let records = stderr
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();

    assert!(!records.is_empty(), "stderr was: {stderr}");
    assert!(
        records.iter().any(|record| {
            record["event"] == "compiler.diagnostic" && record["path"] == "main.typ"
        }),
        "records were: {records:#?}"
    );
    assert!(!stderr.contains("#let broken ="));
    assert!(!stderr.contains("\u{1b}["));
}
