//! The server as it is run: its binary started as a process, configured
//! by nothing but its environment, answering HTTP on a port of its own,
//! and stopped by a signal.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const BINARY: &str = env!("CARGO_BIN_EXE_git-runner-swarm");

/// The panel's server, started with runners kept in memory on a port the
/// system picks, plus whatever `variables` say.
struct Server {
    child: Child,
    address: String,
    log: Arc<Mutex<Vec<String>>>,
}

impl Server {
    fn start(variables: &[(&str, &str)]) -> Self {
        let mut child = command(variables).spawn().unwrap();
        let mut lines = BufReader::new(child.stderr.take().unwrap()).lines();
        let log = Arc::new(Mutex::new(Vec::new()));

        // The port is the one the server says it serves on.
        let address = loop {
            let line = lines
                .next()
                .expect("the server exited before it served anything")
                .unwrap();
            log.lock().unwrap().push(line.clone());
            if let Some(at) = line.find("address=") {
                break line[at + "address=".len()..].trim().to_owned();
            }
        };

        // Keep reading what it logs, so its pipe never fills.
        let rest = Arc::clone(&log);
        thread::spawn(move || {
            for line in lines.map_while(Result::ok) {
                rest.lock().unwrap().push(line);
            }
        });

        Self {
            child,
            address,
            log,
        }
    }

    fn request(&self, method: &str, path: &str, body: Option<&str>) -> (u16, String) {
        let mut stream = TcpStream::connect(&self.address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let body = body.unwrap_or("");
        write!(
            stream,
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\
             Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            self.address,
            body.len()
        )
        .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        let status = response[9..12].parse().unwrap();
        let body = response
            .split_once("\r\n\r\n")
            .map(|(_, body)| body.to_owned())
            .unwrap_or_default();
        (status, body)
    }

    /// Sends SIGTERM, as `docker stop` does, and waits for the server to
    /// exit.
    fn terminate(&mut self) -> ExitStatus {
        let sent = Command::new("kill")
            .args(["-TERM", &self.child.id().to_string()])
            .status()
            .unwrap();
        assert!(sent.success());
        self.child.wait().unwrap()
    }

    fn logged(&self, text: &str) -> bool {
        self.log
            .lock()
            .unwrap()
            .iter()
            .any(|line| line.contains(text))
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The server's command, with an environment of its own and nothing of the
/// test's: what a container would give it.
fn command(variables: &[(&str, &str)]) -> Command {
    let mut command = Command::new(BINARY);
    command
        .env_clear()
        .env("GRS_LISTEN", "127.0.0.1:0")
        .env("GRS_RUNTIME", "memory")
        .envs(variables.iter().copied())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    command
}

/// Runs a server that is expected not to start, and answers how it exited
/// and what it logged.
fn refused(variables: &[(&str, &str)]) -> (ExitStatus, String) {
    let output = command(variables).output().unwrap();
    (
        output.status,
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn serves_the_api_on_the_address_it_is_told() {
    let server = Server::start(&[]);

    let (status, body) = server.request("GET", "/api/health", None);

    assert_eq!(status, 200);
    assert_eq!(body, r#"{"status":"ok"}"#);
}

#[test]
fn creates_and_lists_runners_with_the_runtime_it_is_told() {
    let server = Server::start(&[]);
    let runner =
        r#"{"url":"https://github.com/otsobide/mindweaver","name":"builder","token":"AESOHC2S"}"#;

    let (created, _) = server.request("POST", "/api/runners", Some(runner));
    let (listed, body) = server.request("GET", "/api/runners?owner=otsobide", None);

    assert_eq!(created, 201);
    assert_eq!(listed, 200);
    assert!(body.contains(r#""name":"builder""#), "{body}");
    assert!(
        body.contains(r#""image":"ghcr.io/actions/actions-runner:latest""#),
        "{body}"
    );
    assert!(server.logged("runners are kept in memory"));
}

#[test]
fn runs_github_runners_from_the_image_it_is_told() {
    let server = Server::start(&[("GRS_GITHUB_IMAGE", "mirror.lan/actions-runner:2.338.2")]);

    let (_, images) = server.request("GET", "/api/images", None);

    assert!(
        images.contains(r#""image":"mirror.lan/actions-runner:2.338.2""#),
        "{images}"
    );
}

#[test]
fn serves_the_web_panel_from_the_directory_it_is_told() {
    let web = tempfile::tempdir().unwrap();
    fs::write(
        web.path().join("index.html"),
        "<title>git-runner-swarm</title>",
    )
    .unwrap();
    let with_panel = Server::start(&[("GRS_WEB_DIR", web.path().to_str().unwrap())]);
    let without = Server::start(&[]);

    let (status, body) = with_panel.request("GET", "/", None);
    let (missing, _) = without.request("GET", "/", None);

    assert_eq!(status, 200);
    assert_eq!(body, "<title>git-runner-swarm</title>");
    assert_eq!(missing, 404);
}

#[test]
fn refuses_to_start_with_a_variable_it_cannot_run_with() {
    let (status, log) = refused(&[("GRS_RUNTIME", "podman")]);

    assert_eq!(status.code(), Some(1));
    assert!(
        log.contains("GRS_RUNTIME is `podman`, which is not docker or memory"),
        "{log}"
    );
}

#[test]
fn refuses_to_start_without_the_docker_engine_it_is_told_to_use() {
    let (status, log) = refused(&[
        ("GRS_RUNTIME", "docker"),
        ("DOCKER_HOST", "unix:///nonexistent/docker.sock"),
    ]);

    assert_eq!(status.code(), Some(1));
    assert!(log.contains("cannot be reached"), "{log}");
}

#[test]
fn stops_cleanly_when_terminated() {
    let mut server = Server::start(&[]);
    assert_eq!(server.request("GET", "/api/health", None).0, 200);

    let status = server.terminate();

    assert!(status.success(), "{status}");
    thread::sleep(Duration::from_millis(100));
    assert!(server.logged("stopping"));
    assert!(server.logged("stopped"));
}
