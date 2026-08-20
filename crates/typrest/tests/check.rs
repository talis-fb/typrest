use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::tempdir;

fn write_project(root: &Path, main: &str) {
    fs::create_dir_all(root.join("templates/invoice")).unwrap();
    fs::write(
        root.join("typrest.toml"),
        "schema_version = \"v1\"\ntemplates_root = \"templates\"\n",
    )
    .unwrap();
    fs::write(root.join("templates/invoice/main.typ"), main).unwrap();
    fs::write(
        root.join("templates/invoice/schema.yml"),
        "schemaVersion: v1\nrequest:\n  body:\n    type: object\n",
    )
    .unwrap();
}

#[test]
fn check_succeeds_without_starting_a_server() {
    let root = tempdir().unwrap();
    write_project(root.path(), "Hello");

    let output = Command::new(env!("CARGO_BIN_EXE_typrest"))
        .current_dir(root.path())
        .args(["check", "--color", "never"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Compiling template `invoice`...")
            && stderr.contains("Compiled template `invoice`.")
            && stderr.contains("Templates ready (1):\n  - invoice"),
        "stderr was: {stderr}"
    );
    assert!(!stderr.contains("server listening"));
}

#[test]
fn check_pretty_prints_annotated_typst_source() {
    let root = tempdir().unwrap();
    write_project(root.path(), "#let broken =");

    let output = Command::new(env!("CARGO_BIN_EXE_typrest"))
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

    let output = Command::new(env!("CARGO_BIN_EXE_typrest"))
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
    assert!(records.iter().any(|record| {
        record["event"] == "template.compilation.started"
            && record["context"] == "check"
            && record["template"] == "invoice"
    }));
    assert!(records.iter().any(|record| {
        record["event"] == "template.compilation.failed"
            && record["context"] == "check"
            && record["template"] == "invoice"
    }));
    assert!(!stderr.contains("#let broken ="));
    assert!(!stderr.contains("\u{1b}["));
}

#[test]
fn check_json_reports_ready_templates() {
    let root = tempdir().unwrap();
    write_project(root.path(), "Hello");

    let output = Command::new(env!("CARGO_BIN_EXE_typrest"))
        .current_dir(root.path())
        .args(["check", "--log-format", "json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let records = String::from_utf8_lossy(&output.stderr)
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();

    assert!(records.iter().any(|record| {
        record["event"] == "template.compilation.completed"
            && record["context"] == "check"
            && record["template"] == "invoice"
            && record["warning_count"] == 0
    }));
    assert!(records.iter().any(|record| {
        record["event"] == "templates.ready"
            && record["context"] == "check"
            && record["template_count"] == 1
    }));
    assert!(
        records.iter().any(|record| {
            record["event"] == "template.ready" && record["template"] == "invoice"
        })
    );
}

#[test]
fn cached_packages_do_not_produce_default_package_output() {
    let root = tempdir().unwrap();
    write_project(
        root.path(),
        "#import \"@preview/fixture:1.0.0\": value\n#value",
    );
    let package_dir = root.path().join(".typrest/packages/preview/fixture/1.0.0");
    fs::create_dir_all(&package_dir).unwrap();
    fs::write(
        package_dir.join("typst.toml"),
        "[package]\nname = \"fixture\"\nversion = \"1.0.0\"\nentrypoint = \"lib.typ\"\n",
    )
    .unwrap();
    fs::write(package_dir.join("lib.typ"), "#let value = [cached]").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_typrest"))
        .current_dir(root.path())
        .args(["check", "--color", "never"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.to_lowercase().contains("package"),
        "stderr was: {stderr}"
    );
}

#[test]
fn check_reports_an_empty_ready_registry_without_compilation_lines() {
    let root = tempdir().unwrap();
    fs::create_dir_all(root.path().join("templates")).unwrap();
    fs::write(
        root.path().join("typrest.toml"),
        "schema_version = \"v1\"\ntemplates_root = \"templates\"\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_typrest"))
        .current_dir(root.path())
        .args(["check", "--color", "never"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Templates ready (0)."),
        "stderr was: {stderr}"
    );
    assert!(
        !stderr.contains("Compiling template"),
        "stderr was: {stderr}"
    );
}
