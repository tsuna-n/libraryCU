use super::*;

fn config(home: &std::path::Path, url: &str, provider: &str) {
    std::fs::create_dir_all(home.join("config/lbc")).unwrap();
    std::fs::write(
        home.join("config/lbc/config.toml"),
        format!("[ai]\nprovider = {provider:?}\nmodel = 'fixture'\nbase_url = {url:?}\n"),
    )
    .unwrap();
}

#[test]
fn normal_doctor_and_missing_credentials_never_connect() {
    let home = tempfile::tempdir().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    for (provider, key, opt_in, status) in [
        ("ollama", None, false, "untested"),
        ("openai", Some("private-doctor-key"), false, "untested"),
        ("openai", None, true, "unavailable"),
    ] {
        config(home.path(), &url, provider);
        let mut command = isolated_lbc(home.path());
        command.args(["doctor", "--json"]);
        if let Some(key) = key {
            command.env("OPENAI_API_KEY", key);
        }
        if opt_in {
            command.arg("--connectivity");
        }
        let output = command.output().unwrap();
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["provider_connectivity"]["status"], status);
        assert_eq!(output.status.success(), status == "untested");
        assert!(!String::from_utf8_lossy(&output.stdout).contains("private-doctor-key"));
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
}

#[test]
fn explicit_connectivity_checks_models_and_reports_http_malformed_and_timeout_errors() {
    for (response, delay, expected) in [
        (
            "HTTP/1.1 200 OK\r\nContent-Length: 11\r\n\r\n{\"data\":[]}",
            0,
            "available",
        ),
        (
            "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n\r\n",
            0,
            "error",
        ),
        ("HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}", 0, "error"),
        ("HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\nx", 0, "error"),
        ("", 2, "error"),
    ] {
        let home = tempfile::tempdir().unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        config(
            home.path(),
            &format!("http://{}/v1", listener.local_addr().unwrap()),
            "openai",
        );
        listener.set_nonblocking(true).unwrap();
        let server = thread::spawn(move || {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(std::time::Instant::now() < deadline);
                        thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(error) => panic!("{error}"),
                }
            };
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut bytes = Vec::new();
            while !bytes.windows(4).any(|b| b == b"\r\n\r\n") {
                let mut buffer = [0; 4096];
                let count = stream.read(&mut buffer).unwrap();
                assert!(count > 0 && bytes.len() < 8192);
                bytes.extend_from_slice(&buffer[..count]);
            }
            let request = String::from_utf8(bytes).unwrap();
            assert!(request.starts_with("GET /v1/models HTTP/1.1"));
            assert!(
                request
                    .to_lowercase()
                    .contains("authorization: bearer private-doctor-key")
            );
            assert!(!request.contains("messages"));
            thread::sleep(std::time::Duration::from_secs(delay));
            let _ = stream.write_all(response.as_bytes());
        });
        let output = isolated_lbc(home.path())
            .args([
                "doctor",
                "--connectivity",
                "--connectivity-timeout",
                "1",
                "--json",
            ])
            .env("OPENAI_API_KEY", "private-doctor-key")
            .output()
            .unwrap();
        server.join().unwrap();
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["provider_connectivity"]["status"], expected);
        assert_eq!(output.status.success(), expected == "available");
        assert!(!String::from_utf8_lossy(&output.stdout).contains("private-doctor-key"));
        assert!(!String::from_utf8_lossy(&output.stderr).contains("private-doctor-key"));
    }
}

#[test]
fn invalid_configuration_and_unreachable_or_credential_urls_are_honest() {
    let home = tempfile::tempdir().unwrap();
    config(home.path(), "not a url", "unsupported");
    let output = isolated_lbc(home.path())
        .args(["doctor", "--connectivity", "--json"])
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(!output.status.success());
    assert_eq!(report["checks"][0]["ok"], false);
    assert_eq!(report["provider_connectivity"]["status"], "unavailable");

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    drop(listener);
    for url in [
        url,
        "http://user:private-doctor-key@127.0.0.1/v1".into(),
        "invalid".into(),
    ] {
        config(home.path(), &url, "ollama");
        let output = isolated_lbc(home.path())
            .args(["doctor", "--connectivity", "--json"])
            .output()
            .unwrap();
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(!output.status.success());
        assert_eq!(report["provider_connectivity"]["status"], "error");
        assert!(!String::from_utf8_lossy(&output.stdout).contains("private-doctor-key"));
    }
}
