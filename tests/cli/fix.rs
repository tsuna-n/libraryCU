use super::*;
use std::path::{Path, PathBuf};

const SOURCE: &str = "fn main() { let value: i32 = \"3\"; }\n";
const LOG: &str = "error[E0308]: mismatched types\n --> main.rs:1:28\n";

fn fixture() -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join("config/lbc")).unwrap();
    // Give scanner commands a local root marker even when /tmp has ancestors
    // with project markers in a sandbox or a CI image.
    std::fs::write(home.path().join("README.md"), "Isolated fixture\n").unwrap();
    std::fs::write(home.path().join("main.rs"), SOURCE).unwrap();
    std::fs::write(home.path().join("error.log"), LOG).unwrap();
    home
}

fn ai_config(home: &Path, address: std::net::SocketAddr, language: &str) {
    std::fs::write(home.join("config/lbc/config.toml"), format!(
        "[ai]\nprovider = \"openai-compat\"\nmodel = \"fixture\"\nbase_url = \"http://{address}/v1\"\n[output]\nlanguage = \"{language}\"\n"
    )).unwrap();
}

fn mock(
    home: &Path,
    content: &str,
    language: &str,
    edit: Option<(PathBuf, String)>,
) -> thread::JoinHandle<serde_json::Value> {
    let listener =
        TcpListener::bind("127.0.0.1:0").expect("mock provider needs loopback permission");
    listener.set_nonblocking(true).unwrap();
    ai_config(home, listener.local_addr().unwrap(), language);
    let body = serde_json::json!({"model":"fixture", "choices":[{"message":{"content":content}}]})
        .to_string();
    thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(std::time::Instant::now() < deadline, "no provider request");
                    thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(error) => panic!("{error}"),
            }
        };
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut bytes = Vec::new();
        let request = loop {
            let mut buffer = [0; 4096];
            let count = stream.read(&mut buffer).unwrap();
            assert!(count > 0 && bytes.len() < 256 * 1024);
            bytes.extend_from_slice(&buffer[..count]);
            if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                let length: usize = String::from_utf8_lossy(&bytes[..end])
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length:")?
                            .trim()
                            .parse()
                            .ok()
                    })
                    .unwrap();
                if bytes.len() >= end + 4 + length {
                    break serde_json::from_slice::<serde_json::Value>(
                        &bytes[end + 4..end + 4 + length],
                    )
                    .unwrap();
                }
            }
        };
        if let Some((path, content)) = edit {
            std::fs::write(path, content).unwrap();
        }
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        request
    })
}

#[test]
fn fix_offline_and_invalid_apply_make_zero_provider_connections() {
    let home = fixture();
    let listener = TcpListener::bind("127.0.0.1:0").expect("loopback required");
    listener.set_nonblocking(true).unwrap();
    ai_config(home.path(), listener.local_addr().unwrap(), "auto");
    let output = isolated_lbc(home.path())
        .args(["fix", "error.log", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["status"], "offline_guidance");
    assert_eq!(report["applied"], false);
    assert!(report["patch"].is_null());
    assert!(
        !report["guidance"]["passages"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let output = isolated_lbc(home.path())
        .args(["fix", "error.log", "--apply"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--ai"));
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert_eq!(
        std::fs::read_to_string(home.path().join("main.rs")).unwrap(),
        SOURCE
    );
    assert!(!home.path().join("data/lbc/history").exists());
}

#[test]
fn fix_help_and_input_failures_are_explicit() {
    let home = fixture();
    let output = isolated_lbc(home.path())
        .args(["fix", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    for flag in ["--ai", "--apply", "--json", "--project", "--stdin"] {
        assert!(help.contains(flag));
    }
    for args in [
        vec!["fix", "missing.log"],
        vec!["fix", "error.log", "--stdin"],
        vec!["fix", "--stdin"],
    ] {
        let output = isolated_lbc(home.path())
            .args(args)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}

#[test]
fn fix_stdin_auto_language_uses_the_complete_current_input() {
    let home = fixture();
    let mut child = isolated_lbc(home.path())
        .args(["fix", "--stdin", "--json"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(format!("{LOG}คำอธิบายเพิ่มเติม\n").as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["guidance"]["language"], "th");
    assert_eq!(report["applied"], false);
}

#[test]
fn fix_ai_preview_and_apply_keep_json_and_localized_text_honest() {
    for (apply, json, language) in [
        (false, true, "en"),
        (true, true, "en"),
        (false, false, "en"),
        (true, false, "th"),
    ] {
        let home = fixture();
        std::fs::write(
            home.path().join("unrelated.txt"),
            "API_KEY=private-unrelated-marker",
        )
        .unwrap();
        std::fs::write(
            home.path().join("note.md"),
            "Use exact E0308 mismatched types guidance. API_KEY=private-note-marker\n",
        )
        .unwrap();
        let added = isolated_lbc(home.path())
            .args([
                "add",
                "--id",
                "fixture",
                "--title",
                "E0308 mismatched types",
                "--file",
                "note.md",
            ])
            .output()
            .unwrap();
        assert!(added.status.success());
        let server = mock(
            home.path(),
            r#"{"before":"\"3\"","after":"3"}"#,
            language,
            None,
        );
        let mut command = isolated_lbc(home.path());
        command.args(["fix", "error.log", "--ai"]);
        if apply {
            command.arg("--apply");
        }
        if json {
            command.arg("--json");
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let request = server.join().unwrap();
        let user = request["messages"][1]["content"].as_str().unwrap();
        assert!(user.contains("builtin:rust-e0308"));
        assert!(user.contains("user:fixture"));
        assert!(user.contains("TARGET"));
        assert!(!user.contains("private-unrelated-marker"));
        assert!(!user.contains("private-note-marker"));
        assert!(user.chars().count() <= librarycube::ai::prompt::MAX_PROMPT_CHARS);
        assert!(
            request["messages"][0]["content"]
                .as_str()
                .unwrap()
                .contains("untrusted")
        );
        if json {
            let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(report["status"], if apply { "applied" } else { "proposed" });
            assert_eq!(report["applied"], apply);
            assert_eq!(report["verification_status"], "unverified");
            assert_eq!(report["patch"]["path"], "main.rs");
        } else {
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(stdout.contains(if language == "th" {
                "ยังไม่ได้รันการตรวจสอบ"
            } else {
                "Proposal only"
            }));
        }
        assert_eq!(
            std::fs::read_to_string(home.path().join("main.rs")).unwrap(),
            if apply {
                SOURCE.replace("\"3\"", "3")
            } else {
                SOURCE.to_owned()
            }
        );
        assert!(!home.path().join("data/lbc/history").exists());
    }
}

#[test]
fn fix_provider_or_validation_failure_retains_offline_json_and_source() {
    for content in [
        "not JSON",
        r#"{"before":"missing","after":"new"}"#,
        r#"{"before":"\"3\"","after":"3","path":"../outside.rs"}"#,
        r#"{"before":"\"3\"","after":"API_KEY=do-not-persist-this"}"#,
        "",
    ] {
        let home = fixture();
        let server = mock(home.path(), content, "en", None);
        let output = isolated_lbc(home.path())
            .args(["fix", "error.log", "--ai", "--apply", "--json"])
            .output()
            .unwrap();
        server.join().unwrap();
        assert!(!output.status.success());
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["status"], "failed");
        assert_eq!(report["applied"], false);
        assert!(
            !report["guidance"]["offline_answer"]
                .as_str()
                .unwrap()
                .is_empty()
        );
        assert!(report["error"].is_string());
        assert!(!String::from_utf8_lossy(&output.stdout).contains("do-not-persist-this"));
        assert!(!String::from_utf8_lossy(&output.stderr).contains("do-not-persist-this"));
        assert_eq!(
            std::fs::read_to_string(home.path().join("main.rs")).unwrap(),
            SOURCE
        );
    }
}

#[test]
fn fix_detects_source_changed_while_provider_was_running() {
    let home = fixture();
    let server = mock(
        home.path(),
        r#"{"before":"\"3\"","after":"3"}"#,
        "en",
        Some((home.path().join("main.rs"), "concurrent user edit\n".into())),
    );
    let output = isolated_lbc(home.path())
        .args(["fix", "error.log", "--ai", "--apply", "--json"])
        .output()
        .unwrap();
    server.join().unwrap();
    assert!(!output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(report["error"].as_str().unwrap().contains("source changed"));
    assert_eq!(
        std::fs::read_to_string(home.path().join("main.rs")).unwrap(),
        "concurrent user edit\n"
    );
}

#[test]
fn unsafe_or_ungrounded_fix_never_connects_to_provider() {
    let home = fixture();
    let listener = TcpListener::bind("127.0.0.1:0").expect("loopback required");
    listener.set_nonblocking(true).unwrap();
    ai_config(home.path(), listener.local_addr().unwrap(), "en");
    for (log, source) in [
        (
            "error[E0308]: mismatched types\n --> ../outside.rs:1:1",
            SOURCE,
        ),
        (LOG, "API_KEY=never-send-source-secret\n"),
        (
            "error: completelynovel frobnicator\n --> main.rs:1:1",
            SOURCE,
        ),
    ] {
        std::fs::write(home.path().join("error.log"), log).unwrap();
        std::fs::write(home.path().join("main.rs"), source).unwrap();
        let output = isolated_lbc(home.path())
            .args(["fix", "error.log", "--ai", "--apply", "--json"])
            .output()
            .unwrap();
        assert!(!output.status.success());
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["applied"], false);
        assert!(!String::from_utf8_lossy(&output.stdout).contains("never-send-source-secret"));
        assert_eq!(
            std::fs::read_to_string(home.path().join("main.rs")).unwrap(),
            source
        );
    }
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn python_package_install_search_explain_and_fix_work_together() {
    let home = fixture();
    let package = Path::new(env!("CARGO_MANIFEST_DIR")).join("packages/python-fastapi-basics");
    let output = isolated_lbc(home.path())
        .args(["knowledge", "install"])
        .arg(&package)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::write(home.path().join("main.py"), "print(missing_name)\n").unwrap();
    std::fs::write(home.path().join("error.log"), "Traceback (most recent call last):\n  File \"main.py\", line 1, in <module>\nNameError: name 'missing_name' is not defined\n").unwrap();
    let output = isolated_lbc(home.path())
        .args(["explain", "error.log", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["diagnostic"]["source"], "python");
    assert_eq!(report["diagnostic"]["file"], "main.py");
    assert!(
        report["project_evidence"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["path"] == "main.py")
    );
    assert!(!report["knowledge"].as_array().unwrap().is_empty());
    let server = mock(
        home.path(),
        r#"{"before":"missing_name","after":"\"hello\""}"#,
        "en",
        None,
    );
    let output = isolated_lbc(home.path())
        .args(["fix", "error.log", "--ai", "--json"])
        .output()
        .unwrap();
    let request = server.join().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        request
            .to_string()
            .contains("package:python-fastapi-basics:")
    );
    assert_eq!(
        std::fs::read_to_string(home.path().join("main.py")).unwrap(),
        "print(missing_name)\n"
    );
    for (language, query) in [
        ("en", "CustomWidgetError frobnication worker.py"),
        (
            "th",
            "Python ข้อผิดพลาด CustomWidgetError frobnication worker.py",
        ),
    ] {
        let output = isolated_lbc(home.path())
            .args(["config", "set", "output.language", language])
            .output()
            .unwrap();
        assert!(output.status.success());
        let output = isolated_lbc(home.path())
            .args(["ask", query, "--json"])
            .output()
            .unwrap();
        assert!(output.status.success());
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["answer_status"], "general_guidance");
        assert_eq!(report["language"], language);
        assert!(
            report["offline_answer"]
                .as_str()
                .unwrap()
                .contains(if language == "th" {
                    "ยังไม่พบหลักฐานเพียงพอ"
                } else {
                    "insufficient evidence"
                })
        );
    }
    let output = isolated_lbc(home.path())
        .args(["search", "ModuleNotFoundError", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("package:python-fastapi-basics:"));
}

#[test]
fn fix_uses_explicit_project_boundary_and_does_not_execute_generated_code() {
    let parent = fixture();
    let child = parent.path().join("child");
    std::fs::create_dir(&child).unwrap();
    std::fs::write(child.join("main.rs"), SOURCE).unwrap();
    // Outside this explicitly selected child, despite a project marker above it.
    std::fs::write(
        child.join("error.log"),
        "error[E0308]: mismatched types\n --> ../main.rs:1:1\n",
    )
    .unwrap();
    let output = isolated_lbc(parent.path())
        .args([
            "fix",
            "child/error.log",
            "--project",
            "child",
            "--ai",
            "--apply",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["applied"], false);
    assert_eq!(
        std::fs::read_to_string(parent.path().join("main.rs")).unwrap(),
        SOURCE
    );

    let replacement = "{ std::fs::write(\"executed-marker\", \"bad\").unwrap(); 3 }";
    let content = serde_json::json!({"before":"\"3\"", "after":replacement}).to_string();
    let server = mock(parent.path(), &content, "en", None);
    let output = isolated_lbc(parent.path())
        .args(["fix", "error.log", "--ai", "--apply", "--json"])
        .output()
        .unwrap();
    server.join().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        std::fs::read_to_string(parent.path().join("main.rs"))
            .unwrap()
            .contains("executed-marker")
    );
    assert!(!parent.path().join("executed-marker").exists());
}
