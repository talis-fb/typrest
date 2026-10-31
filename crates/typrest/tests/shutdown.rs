#![cfg(unix)]

use std::fs;
use std::io::Write;
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use tempfile::tempdir;

const LISTEN_DEADLINE: Duration = Duration::from_secs(10);
const EXIT_DEADLINE: Duration = Duration::from_secs(10);
const POLL_INTERVAL: Duration = Duration::from_millis(20);
/// Time for a request to be admitted by the server before the signal is sent.
const ADMISSION_GRACE: Duration = Duration::from_millis(500);

/// A template whose render time is controlled by the payload field `n` (warmup uses `n = 1`).
const PAYLOAD_LOOP_TEMPLATE: &str = r#"#let data = json(bytes(sys.inputs.data))
#let n = data.at("n", default: 1)
#for i in range(n) [#i ]
"#;

/// A template that is slow to warm up, so a signal can arrive during startup.
const SLOW_WARMUP_TEMPLATE: &str = "#for i in range(50000000) [#i ]\n";

struct Server {
    child: Child,
    port: u16,
}

fn unused_port() -> u16 {
    TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn write_project(root: &Path, server_table: &str, template: &str) {
    fs::create_dir_all(root.join("templates/invoice")).unwrap();
    fs::write(
        root.join("typrest.toml"),
        format!("schema_version = \"v1\"\ntemplates_root = \"templates\"\n{server_table}"),
    )
    .unwrap();
    fs::write(root.join("templates/invoice/main.typ"), template).unwrap();
}

fn spawn_server(root: &Path, extra_args: &[&str]) -> Server {
    let port = unused_port();
    let child = Command::new(env!("CARGO_BIN_EXE_typrest"))
        .env_remove("RUST_LOG")
        .current_dir(root)
        .args([
            "serve",
            "--hostname",
            "127.0.0.1",
            "--port",
            &port.to_string(),
            "--color",
            "never",
            "--log-format",
            "json",
        ])
        .args(extra_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    Server { child, port }
}

impl Server {
    fn wait_until_listening(&mut self) {
        let address = SocketAddrV4::new(Ipv4Addr::LOCALHOST, self.port);
        let start = Instant::now();
        while start.elapsed() < LISTEN_DEADLINE {
            if TcpStream::connect(address).is_ok() {
                return;
            }
            assert!(
                self.child.try_wait().unwrap().is_none(),
                "server exited before listening"
            );
            thread::sleep(POLL_INTERVAL);
        }
        self.child.kill().unwrap();
        panic!("server did not listen before the deadline");
    }

    fn signal(&self, name: &str) {
        let status = Command::new("kill")
            .args([&format!("-{name}"), &self.child.id().to_string()])
            .status()
            .unwrap();
        assert!(status.success());
    }

    fn wait_for_exit(mut self, deadline: Duration) -> (ExitStatus, Vec<serde_json::Value>) {
        let start = Instant::now();
        let status = loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                break status;
            }
            if start.elapsed() > deadline {
                self.child.kill().unwrap();
                panic!("server did not exit within {deadline:?}");
            }
            thread::sleep(POLL_INTERVAL);
        };
        let output = self.child.wait_with_output().unwrap();
        assert!(output.stderr.is_empty());
        let records = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();

        (status, records)
    }
}

fn has_event(records: &[serde_json::Value], event: &str) -> bool {
    records.iter().any(|record| record["event"] == event)
}

fn assert_clean_shutdown(signal: &str, extra_args: &[&str]) {
    let root = tempdir().unwrap();
    write_project(root.path(), "", "Hello");
    let mut server = spawn_server(root.path(), extra_args);
    server.wait_until_listening();

    server.signal(signal);
    let (status, records) = server.wait_for_exit(EXIT_DEADLINE);

    assert_eq!(status.code(), Some(0));
    let started = records
        .iter()
        .find(|record| record["event"] == "server.shutdown.started")
        .expect("server.shutdown.started");
    assert_eq!(started["signal"], format!("SIG{signal}"));
    assert!(has_event(&records, "server.shutdown.completed"));
    assert!(!has_event(&records, "server.shutdown.timed_out"));
}

#[test]
fn sigterm_drains_and_exits_successfully() {
    assert_clean_shutdown("TERM", &[]);
}

#[test]
fn sigint_drains_and_exits_successfully() {
    assert_clean_shutdown("INT", &[]);
}

#[test]
fn shutdown_with_watch_mode_exits_successfully() {
    assert_clean_shutdown("TERM", &["--watch"]);
}

#[test]
fn signal_during_warmup_aborts_startup_with_failure() {
    let root = tempdir().unwrap();
    write_project(root.path(), "", SLOW_WARMUP_TEMPLATE);
    let server = spawn_server(root.path(), &[]);
    thread::sleep(ADMISSION_GRACE);

    server.signal("TERM");
    let (status, records) = server.wait_for_exit(EXIT_DEADLINE);

    assert_eq!(status.code(), Some(1));
    assert!(records.iter().any(|record| {
        record["event"] == "server.shutdown.started" && record["context"] == "startup"
    }));
    assert!(!has_event(&records, "server.listening"));
}

#[test]
fn stuck_render_times_out_and_exits_without_waiting_for_it() {
    let root = tempdir().unwrap();
    write_project(
        root.path(),
        "[server]\nshutdown_timeout_seconds = 1\n",
        PAYLOAD_LOOP_TEMPLATE,
    );
    let mut server = spawn_server(root.path(), &[]);
    server.wait_until_listening();

    let body = r#"{"n": 100000000}"#;
    let mut request = TcpStream::connect((Ipv4Addr::LOCALHOST, server.port)).unwrap();
    write!(
        request,
        "POST /invoice HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .unwrap();
    thread::sleep(ADMISSION_GRACE);

    let signalled = Instant::now();
    server.signal("TERM");
    let (status, records) = server.wait_for_exit(Duration::from_secs(5));

    assert_eq!(status.code(), Some(1));
    assert!(signalled.elapsed() < Duration::from_secs(4));
    assert!(has_event(&records, "server.shutdown.started"));
    assert!(has_event(&records, "server.shutdown.timed_out"));
    assert!(!has_event(&records, "server.shutdown.completed"));
}
