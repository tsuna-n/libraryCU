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
            stream.set_nonblocking(false).unwrap();
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

#[test]
fn fresh_doctor_explains_optional_stores_without_creating_them_or_changing_json_schema() {
    let home = tempfile::tempdir().unwrap();
    let output = isolated_lbc(home.path())
        .args(["doctor", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let mut keys: Vec<_> = report
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["checks", "health_scope", "healthy", "provider_connectivity"]
    );
    assert!(report["healthy"].is_boolean());
    assert!(report["health_scope"].is_string());
    assert_eq!(
        report["provider_connectivity"].as_object().unwrap().len(),
        2
    );
    let checks = report["checks"].as_array().unwrap();
    assert_eq!(
        checks
            .iter()
            .map(|check| check["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "Configuration",
            "Project detection",
            "Local knowledge",
            "AI provider",
            "Session memory"
        ]
    );
    for check in checks {
        assert_eq!(check.as_object().unwrap().len(), 3);
        assert!(check["name"].is_string());
        assert!(check["ok"].is_boolean());
        assert!(check["detail"].is_string());
    }
    let knowledge = checks
        .iter()
        .find(|check| check["name"] == "Local knowledge")
        .unwrap();
    assert!(
        knowledge["detail"]
            .as_str()
            .unwrap()
            .contains("not created")
    );
    assert!(
        report["checks"][0]["detail"]
            .as_str()
            .unwrap()
            .contains("lbc setup")
    );
    assert_eq!(std::fs::read_dir(home.path()).unwrap().count(), 0);
}

#[test]
fn doctor_reports_storage_files_as_problems_without_repairing_them() {
    for store in [
        "data/lbc/notes",
        "data/lbc/knowledge",
        ".lbc/knowledge",
        "knowledge",
        ".lbc",
    ] {
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join(store);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "preserve this file").unwrap();
        let output = isolated_lbc(home.path())
            .args(["doctor", "--json"])
            .output()
            .unwrap();
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(!output.status.success(), "{store}: {report}");
        let check = report["checks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|check| check["name"] == "Local knowledge")
            .unwrap();
        assert_eq!(check["ok"], false);
        let detail = check["detail"].as_str().unwrap();
        assert!(detail.contains("directory"), "{detail}");
        assert!(detail.contains("lbc doctor"), "{detail}");
        assert_eq!(std::fs::read_to_string(path).unwrap(), "preserve this file");
    }
}

#[test]
fn doctor_explains_invalid_config_without_echoing_toml_source() {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join("config/lbc")).unwrap();
    let path = home.path().join("config/lbc/config.toml");
    let source = "[output]\nlanguage = [\"private-unlabelled-secret\"]\n";
    std::fs::write(&path, source).unwrap();
    let output = isolated_lbc(home.path())
        .args(["doctor", "--json"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let detail = report["checks"][0]["detail"].as_str().unwrap();
    assert!(detail.contains("TOML"), "{detail}");
    assert!(detail.contains("lbc doctor"), "{detail}");
    assert!(!String::from_utf8_lossy(&output.stdout).contains("private-unlabelled-secret"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("private-unlabelled-secret"));
    assert_eq!(std::fs::read_to_string(path).unwrap(), source);
}

#[cfg(unix)]
#[test]
fn doctor_rejects_dangling_config_and_knowledge_symlinks() {
    for store in [
        "config/lbc/config.toml",
        "data/lbc/notes",
        "data/lbc/knowledge",
        ".lbc",
    ] {
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join(store);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(home.path().join("missing-target"), &path).unwrap();
        let output = isolated_lbc(home.path())
            .args(["doctor", "--json"])
            .output()
            .unwrap();
        assert!(!output.status.success(), "{store}");
        assert!(String::from_utf8_lossy(&output.stdout).contains("symlink"));
        assert!(
            std::fs::symlink_metadata(path)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert!(!home.path().join("missing-target").exists());
    }
}

#[test]
fn doctor_uses_configured_thai_and_offers_stable_next_commands() {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join("config/lbc")).unwrap();
    for language in ["th", "auto"] {
        std::fs::write(
            home.path().join("config/lbc/config.toml"),
            format!("[output]\nlanguage = '{language}'\n[ai]\nprovider = 'openai'\n"),
        )
        .unwrap();
        let output = isolated_lbc(home.path())
            .arg("doctor")
            .env("LC_ALL", "th_TH.UTF-8")
            .output()
            .unwrap();
        assert!(!output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("การตั้งค่า"), "{language}: {stdout}");
        assert!(stdout.contains("วิธีแก้"), "{language}: {stdout}");
        assert!(stdout.contains("lbc config show"), "{stdout}");
        assert!(stdout.contains("lbc doctor --connectivity"), "{stdout}");
    }
}

#[test]
fn doctor_does_not_echo_invalid_provider_values() {
    let home = tempfile::tempdir().unwrap();
    config(home.path(), "", "private-unlabelled-provider-value");
    let output = isolated_lbc(home.path())
        .args(["doctor", "--json"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("private-unlabelled-provider-value"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("private-unlabelled-provider-value"));
}

#[test]
fn doctor_reports_invalid_documents_and_preserves_existing_stores_and_history() {
    let home = tempfile::tempdir().unwrap();
    let files = [
        ("config/lbc/config.toml", "[memory]\nmode = 'persistent'\n"),
        ("data/lbc/notes/broken.md", "missing frontmatter\n"),
        ("data/lbc/history/default.json", "private history fixture\n"),
        ("cache/lbc/index.sqlite", "preserve index fixture\n"),
    ];
    for (path, content) in files {
        let path = home.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }
    let output = isolated_lbc(home.path())
        .args(["doctor", "--json"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let check = report["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|check| check["name"] == "Local knowledge")
        .unwrap();
    let detail = check["detail"].as_str().unwrap();
    assert!(detail.contains("broken.md"), "{detail}");
    assert!(detail.contains("missing YAML frontmatter"), "{detail}");
    assert!(detail.contains("lbc index --json"), "{detail}");
    assert!(!String::from_utf8_lossy(&output.stdout).contains("private history fixture"));
    for (path, content) in files {
        assert_eq!(
            std::fs::read_to_string(home.path().join(path)).unwrap(),
            content
        );
    }
    assert!(!home.path().join("data/lbc/history/default.lock").exists());
    assert!(!home.path().join("data/lbc/knowledge").exists());
}

#[cfg(unix)]
#[test]
fn doctor_reports_unreadable_config_and_knowledge_stores() {
    use std::os::unix::fs::PermissionsExt;
    // Root can read mode-000 paths; permission failures require an ordinary user.
    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    for store in [
        "config/lbc/config.toml",
        "data/lbc/notes",
        "data/lbc/knowledge",
        ".lbc/knowledge",
    ] {
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join(store);
        if store.ends_with(".toml") {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, "[ai]\nprovider = 'off'\n").unwrap();
        } else {
            std::fs::create_dir_all(&path).unwrap();
        }
        let permissions = std::fs::metadata(&path).unwrap().permissions();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
        let output = isolated_lbc(home.path())
            .args(["doctor", "--json"])
            .output()
            .unwrap();
        std::fs::set_permissions(&path, permissions).unwrap();
        assert!(!output.status.success(), "{store}");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Permission denied"), "{stdout}");
        assert!(stdout.contains("lbc doctor"), "{stdout}");
    }
}
