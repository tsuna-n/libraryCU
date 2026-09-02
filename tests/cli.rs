use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn lbc() -> Command {
    Command::new(env!("CARGO_BIN_EXE_lbc"))
}

#[test]
fn help_lists_v01_commands() {
    let output = lbc().arg("--help").output().expect("lbc should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    for command in ["scan", "explain", "search", "config", "doctor"] {
        assert!(stdout.contains(command), "help did not list {command}");
    }
    assert!(stdout.contains("LibraryCube - Developer Diagnostic Toolkit"));
}

#[test]
fn explain_help_documents_pipeline_options() {
    let output = lbc()
        .args(["explain", "--help"])
        .output()
        .expect("lbc explain --help should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    for option in ["--stdin", "--verbose", "--json"] {
        assert!(stdout.contains(option), "help did not list {option}");
    }
    assert!(stdout.contains("[FILE]"));
}

#[test]
fn config_show_uses_defaults_when_file_is_missing() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let missing = std::env::temp_dir().join(format!("lbc-missing-{nonce}/config.toml"));
    let output = lbc()
        .args(["config", "show"])
        .env("LBC_CONFIG", missing)
        .output()
        .expect("lbc config show should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Using default configuration"));
    assert!(stdout.contains("Language:        auto"));
    assert!(stdout.contains("Max file size:   256 KB"));
}

#[test]
fn config_set_shows_and_persists_the_requested_change() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let directory = std::env::temp_dir().join(format!("lbc-config-set-{nonce}"));
    let config_path = directory.join("config.toml");
    let output = lbc()
        .args(["config", "set", "scanner.max_file_size_kb", "512"])
        .env("LBC_CONFIG", &config_path)
        .output()
        .expect("lbc config set should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Proposed configuration change"));
    assert!(stdout.contains("scanner.max_file_size_kb = 512"));

    let output = lbc()
        .args(["config", "show", "--json"])
        .env("LBC_CONFIG", &config_path)
        .output()
        .expect("lbc config show should read the saved value");
    assert!(output.status.success());
    let config: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("config output should be valid JSON");
    assert_eq!(config["scanner"]["max_file_size_kb"], 512);
    std::fs::remove_dir_all(directory).expect("temporary config directory should be removable");
}

#[test]
fn scan_detects_this_rust_project() {
    let output = lbc()
        .args(["scan", "--path", env!("CARGO_MANIFEST_DIR")])
        .output()
        .expect("lbc scan should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Project detected"));
    assert!(stdout.contains("Rust"));
    assert!(stdout.contains("Cargo"));
    assert!(stdout.contains("Scan Summary"));
}

#[test]
fn search_finds_exact_rust_error_code() {
    let output = lbc()
        .args(["search", "E0382"])
        .output()
        .expect("lbc search should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Rust E0382"));
    assert!(stdout.contains("exact error code"));
}

#[test]
fn explain_reads_rust_error_from_a_pipeline() {
    let mut child = lbc()
        .arg("explain")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("lbc explain should start");
    let mut stdin = child.stdin.take().expect("stdin should be piped");
    stdin
        .write_all(
            b"error[E0382]: borrow of moved value: `name`\n --> src/main.rs:10:5\n value moved here\n",
        )
        .expect("error input should be written");
    drop(stdin);
    let output = child.wait_with_output().expect("lbc explain should finish");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("LBC Diagnostic"));
    assert!(stdout.contains("E0382"));
    assert!(stdout.contains("Evidence"));
    assert!(stdout.contains("Suggested fix"));
    assert!(stdout.contains("cargo check"));
    assert!(stdout.contains("Known diagnostic rule"));
}

#[test]
fn doctor_checks_local_components_without_ai() {
    let output = lbc().arg("doctor").output().expect("lbc doctor should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Configuration"));
    assert!(stdout.contains("Project detection"));
    assert!(stdout.contains("Local knowledge"));
    assert!(stdout.contains("AI provider"));
    assert!(stdout.contains("deterministic mode"));
    assert!(stdout.contains("healthy"));
}

fn write_ai_config(provider: &str, extra: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let directory = std::env::temp_dir().join(format!("lbc-ai-{provider}-{nonce}"));
    std::fs::create_dir_all(&directory).expect("temporary config directory should be creatable");
    let path = directory.join("config.toml");
    std::fs::write(
        &path,
        format!("[ai]\nprovider = \"{provider}\"\nmodel = \"test-model\"\n{extra}"),
    )
    .expect("temporary config should be writable");
    path
}

#[test]
fn explain_help_documents_the_ai_flag() {
    let output = lbc()
        .args(["explain", "--help"])
        .output()
        .expect("lbc explain --help should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--ai"), "help did not document --ai");
}

#[test]
fn config_show_reports_the_ai_section() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let missing = std::env::temp_dir().join(format!("lbc-ai-default-{nonce}/config.toml"));
    let output = lbc()
        .args(["config", "show"])
        .env("LBC_CONFIG", missing)
        .output()
        .expect("lbc config show should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Provider:        off"));
}

#[test]
fn explain_ai_falls_back_to_the_deterministic_report_when_the_provider_is_unreachable() {
    let config_path = write_ai_config("openai-compat", "base_url = \"http://127.0.0.1:9/v1\"\n");
    let mut child = lbc()
        .args(["explain", "--ai"])
        .env("LBC_CONFIG", &config_path)
        .env_remove("OPENAI_API_KEY")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("lbc explain --ai should start");
    let mut stdin = child.stdin.take().expect("stdin should be piped");
    stdin
        .write_all(
            b"error[E0382]: borrow of moved value: `name`\n --> src/main.rs:10:5\n value moved here\n",
        )
        .expect("error input should be written");
    drop(stdin);
    let output = child
        .wait_with_output()
        .expect("lbc explain --ai should finish");
    assert!(
        output.status.success(),
        "fallback must keep the command successful"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("LBC Diagnostic"));
    assert!(stdout.contains("Known diagnostic rule"));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("AI enhancement failed"),
        "stderr should explain the fallback: {stderr}"
    );
    std::fs::remove_dir_all(config_path.parent().unwrap())
        .expect("temporary config directory should be removable");
}

#[test]
fn explain_json_omits_ai_when_not_requested() {
    let mut child = lbc()
        .args(["explain", "--json"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("lbc explain should start");
    let mut stdin = child.stdin.take().expect("stdin should be piped");
    stdin
        .write_all(b"error[E0382]: borrow of moved value: `name`\n value moved here\n")
        .expect("error input should be written");
    drop(stdin);
    let output = child.wait_with_output().expect("lbc explain should finish");
    assert!(output.status.success());
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("report should be valid JSON");
    assert!(
        report.get("ai").is_none(),
        "ai must stay omitted without --ai"
    );
    assert_eq!(report["confidence"], "known_rule");
}
