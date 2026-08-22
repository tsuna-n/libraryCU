use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn lcu() -> Command {
    Command::new(env!("CARGO_BIN_EXE_lcu"))
}

#[test]
fn help_lists_v01_commands() {
    let output = lcu().arg("--help").output().expect("lcu should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    for command in ["scan", "explain", "search", "config", "doctor"] {
        assert!(stdout.contains(command), "help did not list {command}");
    }
    assert!(stdout.contains("LibraryCU - Developer Diagnostic Toolkit"));
}

#[test]
fn explain_help_documents_pipeline_options() {
    let output = lcu()
        .args(["explain", "--help"])
        .output()
        .expect("lcu explain --help should run");
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
    let missing = std::env::temp_dir().join(format!("lcu-missing-{nonce}/config.toml"));
    let output = lcu()
        .args(["config", "show"])
        .env("LCU_CONFIG", missing)
        .output()
        .expect("lcu config show should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Using default configuration"));
    assert!(stdout.contains("Language:        auto"));
    assert!(stdout.contains("Max file size:   256 KB"));
}

#[test]
fn scan_detects_this_rust_project() {
    let output = lcu()
        .args(["scan", "--path", env!("CARGO_MANIFEST_DIR")])
        .output()
        .expect("lcu scan should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Project detected"));
    assert!(stdout.contains("Rust"));
    assert!(stdout.contains("Cargo"));
    assert!(stdout.contains("Scan Summary"));
}

#[test]
fn search_finds_exact_rust_error_code() {
    let output = lcu()
        .args(["search", "E0382"])
        .output()
        .expect("lcu search should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Rust E0382"));
    assert!(stdout.contains("exact error code"));
}

#[test]
fn explain_reads_rust_error_from_a_pipeline() {
    let mut child = lcu()
        .arg("explain")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("lcu explain should start");
    let mut stdin = child.stdin.take().expect("stdin should be piped");
    stdin
        .write_all(
            b"error[E0382]: borrow of moved value: `name`\n --> src/main.rs:10:5\n value moved here\n",
        )
        .expect("error input should be written");
    drop(stdin);
    let output = child.wait_with_output().expect("lcu explain should finish");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("LCU Diagnostic"));
    assert!(stdout.contains("E0382"));
    assert!(stdout.contains("Evidence"));
    assert!(stdout.contains("Suggested fix"));
    assert!(stdout.contains("cargo check"));
    assert!(stdout.contains("Known diagnostic rule"));
}

#[test]
fn doctor_checks_local_components_without_ai() {
    let output = lcu().arg("doctor").output().expect("lcu doctor should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Configuration"));
    assert!(stdout.contains("Project detection"));
    assert!(stdout.contains("Local knowledge"));
    assert!(stdout.contains("healthy"));
}
