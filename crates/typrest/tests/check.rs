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
        stderr
            == "Compiling template `invoice`\n Compiled template `invoice`\n\nTemplates ready: 1\n",
        "stderr was: {stderr}"
    );
    assert!(output.stdout.is_empty());
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
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("error:"));
    assert!(stdout.contains("templates/invoice/main.typ"));
    assert!(stdout.contains("#let broken ="));
    assert!(stdout.contains('^'));
    assert!(stdout.contains("ERROR template compilation failed"));
    assert!(stderr.contains("Compiling template `invoice`"));
    assert!(!stderr.contains("error:"));
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
    let stdout = String::from_utf8_lossy(&output.stdout);
    let feedback_records = stderr
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();
    let error_records = stdout
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();

    assert!(!feedback_records.is_empty(), "stderr was: {stderr}");
    assert!(
        error_records.iter().any(|record| {
            record["event"] == "compiler.diagnostic" && record["path"] == "main.typ"
        }),
        "records were: {error_records:#?}"
    );
    assert!(feedback_records.iter().any(|record| {
        record["event"] == "template.compilation.started"
            && record["context"] == "check"
            && record["template"] == "invoice"
    }));
    assert!(error_records.iter().any(|record| {
        record["event"] == "template.compilation.failed"
            && record["context"] == "check"
            && record["template"] == "invoice"
    }));
    assert!(!stderr.contains("#let broken ="));
    assert!(!stdout.contains("#let broken ="));
    assert!(!stderr.contains("\u{1b}["));
    assert!(!stdout.contains("\u{1b}["));
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
    assert!(output.stdout.is_empty());
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
fn check_writes_prebuild_warnings_to_stderr() {
    let root = tempdir().unwrap();
    write_project(root.path(), "<orphan>");

    let output = Command::new(env!("CARGO_BIN_EXE_typrest"))
        .current_dir(root.path())
        .args(["check", "--color", "never"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("warning:"), "stderr was: {stderr}");
    assert!(stderr.contains("not attached"), "stderr was: {stderr}");
    assert!(output.stdout.is_empty());

    let json_output = Command::new(env!("CARGO_BIN_EXE_typrest"))
        .current_dir(root.path())
        .args(["check", "--log-format", "json"])
        .output()
        .unwrap();
    let records = String::from_utf8_lossy(&json_output.stderr)
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert!(
        records.iter().any(|record| {
            record["event"] == "compiler.diagnostic" && record["level"] == "WARN"
        })
    );
    assert!(json_output.stdout.is_empty());
}

#[test]
fn check_reports_multiple_templates_in_order_with_a_count_only_summary() {
    let root = tempdir().unwrap();
    write_project(root.path(), "Hello");
    for template in ["resume", "support/thanks-cart", "support/welcome-doc"] {
        let template_dir = root.path().join("templates").join(template);
        fs::create_dir_all(&template_dir).unwrap();
        fs::write(template_dir.join("main.typ"), "Hello").unwrap();
    }

    let output = Command::new(env!("CARGO_BIN_EXE_typrest"))
        .current_dir(root.path())
        .args(["check", "--color", "never"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Compiling template `invoice`\n Compiled template `invoice`\nCompiling template `resume`\n Compiled template `resume`\nCompiling template `support/thanks-cart`\n Compiled template `support/thanks-cart`\nCompiling template `support/welcome-doc`\n Compiled template `support/welcome-doc`\n\nTemplates ready: 4\n"
    );
    assert!(output.stdout.is_empty());
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
    assert!(stderr == "Templates ready: 0\n", "stderr was: {stderr}");
    assert!(
        !stderr.contains("Compiling template"),
        "stderr was: {stderr}"
    );
}
