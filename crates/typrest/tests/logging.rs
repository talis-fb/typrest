use std::fs;
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::Duration;

use tempfile::tempdir;

fn write_project(root: &Path) {
    fs::create_dir_all(root.join("templates/invoice")).unwrap();
    fs::write(
        root.join("typrest.toml"),
        "schema_version = \"v1\"\ntemplates_root = \"templates\"\n",
    )
    .unwrap();
    fs::write(root.join("templates/invoice/main.typ"), "Hello").unwrap();
}

fn unused_port() -> u16 {
    TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn run_server(root: &Path, format: &str) -> Output {
    let port = unused_port();
    let mut child = Command::new(env!("CARGO_BIN_EXE_typrest"))
        .current_dir(root)
        .args([
            "serve",
            "--watch",
            "--hostname",
            "127.0.0.1",
            "--port",
            &port.to_string(),
            "--color",
            "never",
            "--log-format",
            format,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let address = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port);
    for _ in 0..100 {
        if TcpStream::connect(address).is_ok() {
            child.kill().unwrap();
            return child.wait_with_output().unwrap();
        }
        if child.try_wait().unwrap().is_some() {
            return child.wait_with_output().unwrap();
        }
        thread::sleep(Duration::from_millis(20));
    }

    child.kill().unwrap();
    panic!("server did not listen before timeout");
}

#[test]
fn pretty_serve_splits_prebuild_feedback_from_backend_logs() {
    let root = tempdir().unwrap();
    write_project(root.path());

    let output = run_server(root.path(), "pretty");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert_eq!(
        stderr,
        "Compiling template `invoice`\n Compiled template `invoice`\n\nTemplates ready: 1\n\n======== Starting server =========\n"
    );
    assert!(stdout.contains(" INFO template watch mode enabled\n"));
    assert!(
        stdout.contains(" INFO typrest server listening on 127.0.0.1:"),
        "stdout was: {stdout}"
    );
    assert!(!stdout.contains("event="));
    assert!(!stdout.contains("address="));
    assert!(!stdout.contains("templates_dir="));
}

#[test]
fn json_serve_keeps_prebuild_feedback_and_backend_records_separate() {
    let root = tempdir().unwrap();
    write_project(root.path());

    let output = run_server(root.path(), "json");
    let feedback = String::from_utf8_lossy(&output.stderr)
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();
    let backend = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();

    assert!(feedback.iter().all(|record| {
        matches!(
            record["event"].as_str(),
            Some(
                "template.compilation.started"
                    | "template.compilation.completed"
                    | "templates.ready"
                    | "template.ready"
            )
        )
    }));
    assert!(backend.iter().any(|record| {
        record["event"] == "template.watch.enabled" && record["templates_dir"].is_string()
    }));
    assert!(
        backend.iter().any(|record| {
            record["event"] == "server.listening" && record["address"].is_string()
        })
    );
}

#[test]
fn server_bind_errors_are_timestamped_stdout_logs() {
    let root = tempdir().unwrap();
    write_project(root.path());
    let occupied = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let port = occupied.local_addr().unwrap().port().to_string();

    let output = Command::new(env!("CARGO_BIN_EXE_typrest"))
        .current_dir(root.path())
        .args([
            "serve",
            "--hostname",
            "127.0.0.1",
            "--port",
            &port,
            "--color",
            "never",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stdout.contains(" ERROR typrest server failed:"),
        "stdout was: {stdout}"
    );
    assert!(!stdout.contains("event="));
    assert!(stderr.ends_with("======== Starting server =========\n"));
    assert!(!stderr.contains("ERROR"));
}
