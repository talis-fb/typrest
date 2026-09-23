use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::tempdir;

fn typrest_command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_typrest"));
    command.env_remove("RUST_LOG");
    command
}

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

    let output = typrest_command()
        .current_dir(root.path())
        .args(["check", "--color", "never"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains(" INFO compiling template `invoice`\n"));
    assert!(stdout.contains(" INFO compiled template `invoice`\n"));
    assert!(stdout.contains(" INFO templates ready: 1\n"));
    assert!(stderr.is_empty(), "stderr was: {stderr}");
    assert!(!stdout.contains("server listening"));
}

#[test]
fn check_pretty_prints_annotated_typst_source() {
    let root = tempdir().unwrap();
    write_project(root.path(), "#let broken =");

    let output = typrest_command()
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
    assert!(stdout.contains("compiling template `invoice`"));
    assert!(stderr.is_empty(), "stderr was: {stderr}");
}

#[test]
fn check_pretty_preserves_call_traces() {
    let root = tempdir().unwrap();
    write_project(root.path(), "#let helper() = tetx(\"x\")\n#helper()");

    let output = typrest_command()
        .current_dir(root.path())
        .args(["check", "--color", "never"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("note:"), "stdout was: {stdout}");
    assert!(output.stderr.is_empty());
}

#[test]
fn check_json_is_jsonl_and_omits_source_excerpts() {
    let root = tempdir().unwrap();
    write_project(root.path(), "#let broken =");

    let output = typrest_command()
        .current_dir(root.path())
        .args(["check", "--log-format", "json"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let records = stdout
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();

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
    assert!(!stdout.contains("#let broken ="));
    assert!(!stdout.contains("\u{1b}["));
    assert!(stderr.is_empty(), "stderr was: {stderr}");
}

#[test]
fn check_json_correlates_diagnostics_and_traces() {
    let root = tempdir().unwrap();
    write_project(root.path(), "#let helper() = missing\n#helper()");

    let output = typrest_command()
        .current_dir(root.path())
        .args(["check", "--log-format", "json"])
        .output()
        .unwrap();

    let records = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();
    let primary = records
        .iter()
        .find(|record| record["event"] == "compiler.diagnostic")
        .unwrap();
    let trace = records
        .iter()
        .find(|record| record["event"] == "compiler.trace")
        .unwrap();

    assert_eq!(trace["report_id"], primary["report_id"]);
    assert!(trace["related_index"].is_number());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("#let helper"));
    assert!(output.stderr.is_empty());
}

#[test]
fn check_json_reports_ready_templates() {
    let root = tempdir().unwrap();
    write_project(root.path(), "Hello");

    let output = typrest_command()
        .current_dir(root.path())
        .args(["check", "--log-format", "json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let records = String::from_utf8_lossy(&output.stdout)
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
    assert!(output.stderr.is_empty());
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

    let output = typrest_command()
        .current_dir(root.path())
        .args(["check", "--color", "never"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.to_lowercase().contains("package"),
        "stdout was: {stdout}"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn check_writes_prebuild_warnings_to_stdout() {
    let root = tempdir().unwrap();
    write_project(root.path(), "<orphan>");

    let output = typrest_command()
        .current_dir(root.path())
        .args(["check", "--color", "never"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("warning:"), "stdout was: {stdout}");
    assert!(stdout.contains("not attached"), "stdout was: {stdout}");
    assert!(output.stderr.is_empty());

    let json_output = typrest_command()
        .current_dir(root.path())
        .args(["check", "--log-format", "json"])
        .output()
        .unwrap();
    let records = String::from_utf8_lossy(&json_output.stdout)
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert!(
        records.iter().any(|record| {
            record["event"] == "compiler.diagnostic" && record["level"] == "WARN"
        })
    );
    assert!(json_output.stderr.is_empty());
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

    let output = typrest_command()
        .current_dir(root.path())
        .args(["check", "--color", "never"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let messages = [
        "compiling template `invoice`",
        "compiled template `invoice`",
        "compiling template `resume`",
        "compiled template `resume`",
        "compiling template `support/thanks-cart`",
        "compiled template `support/thanks-cart`",
        "compiling template `support/welcome-doc`",
        "compiled template `support/welcome-doc`",
        "templates ready: 4",
    ];
    let mut previous = 0;
    for message in messages {
        let position = stdout[previous..].find(message).unwrap() + previous;
        previous = position + message.len();
    }
    assert!(output.stderr.is_empty());
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

    let output = typrest_command()
        .current_dir(root.path())
        .args(["check", "--color", "never"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(" INFO templates ready: 0\n"));
    assert!(
        !stdout.contains("compiling template"),
        "stdout was: {stdout}"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn rust_log_off_suppresses_tracing_but_not_pretty_compiler_diagnostics() {
    let root = tempdir().unwrap();
    write_project(root.path(), "#let broken =");

    let output = typrest_command()
        .current_dir(root.path())
        .env("RUST_LOG", "off")
        .args(["check", "--color", "never"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("#let broken ="));
    assert!(stdout.contains('^'));
    assert!(!stdout.contains("compiling template"));
    assert!(output.stderr.is_empty());
}

#[test]
fn pretty_compiler_diagnostics_apply_stdout_color_policy() {
    let root = tempdir().unwrap();
    write_project(root.path(), "#let broken =");

    let output = typrest_command()
        .current_dir(root.path())
        .args(["check", "--color", "always"])
        .output()
        .unwrap();

    assert!(String::from_utf8_lossy(&output.stdout).contains("\u{1b}["));
    assert!(output.stderr.is_empty());
}

#[test]
fn cached_packages_are_logged_at_debug() {
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

    let output = typrest_command()
        .current_dir(root.path())
        .env("RUST_LOG", "typrest=debug")
        .args(["check", "--color", "never"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(" DEBUG using cached package `@preview/fixture:1.0.0`"));
    assert!(output.stderr.is_empty());
}
