use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::sync::{LazyLock, mpsc};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

static DEFAULT_TEST_HOME: LazyLock<std::path::PathBuf> =
    LazyLock::new(|| std::env::temp_dir().join(format!("lbc-test-default-{}", std::process::id())));

fn lbc() -> Command {
    let binary =
        std::env::var_os("LBC_TEST_BINARY").unwrap_or_else(|| env!("CARGO_BIN_EXE_lbc").into());
    let mut command = Command::new(binary);
    command
        .env("XDG_DATA_HOME", DEFAULT_TEST_HOME.join("data"))
        .env("XDG_CONFIG_HOME", DEFAULT_TEST_HOME.join("config"))
        .env("XDG_CACHE_HOME", DEFAULT_TEST_HOME.join("cache"))
        .env_remove("LBC_CONFIG")
        .env_remove("OPENROUTER_API_KEY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("GLM_API_KEY")
        .env_remove("ZAI_API_KEY");
    command
}

fn isolated_lbc(home: &std::path::Path) -> Command {
    let mut command = lbc();
    command
        .current_dir(home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_CACHE_HOME", home.join("cache"))
        .env_remove("LBC_CONFIG")
        .env_remove("OPENROUTER_API_KEY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("GLM_API_KEY")
        .env_remove("ZAI_API_KEY");
    command
}

fn temporary_home(name: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!("lbc-{name}-{}-{nonce}", std::process::id()))
}

#[test]
fn help_lists_v01_commands() {
    let output = lbc().arg("--help").output().expect("lbc should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    for command in ["scan", "explain", "search", "config", "doctor"] {
        assert!(stdout.contains(command), "help did not list {command}");
    }
    assert!(stdout.contains("libraryCube - terminal knowledge library"));
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
    assert!(stdout.contains("libraryCube diagnostic"));
    assert!(stdout.contains("E0382"));
    assert!(stdout.contains("Evidence"));
    assert!(stdout.contains("Suggested fix"));
    assert!(stdout.contains("cargo check"));
    assert!(stdout.contains("Known diagnostic rule"));
}

#[test]
fn doctor_checks_local_components_without_ai() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let missing = std::env::temp_dir().join(format!("lbc-doctor-missing-{nonce}/config.toml"));
    let output = lbc()
        .arg("doctor")
        .env("LBC_CONFIG", missing)
        .output()
        .expect("lbc doctor should run");
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
    assert!(stdout.contains("libraryCube diagnostic"));
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

#[test]
fn help_lists_the_knowledge_command() {
    let output = lbc().arg("--help").output().expect("lbc --help should run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("knowledge"), "help did not list knowledge");
}

#[test]
fn search_covers_linux_git_and_docker_knowledge() {
    for (query, expected) in [
        ("merge conflict", "Git - Merge conflict"),
        ("detached head", "Git - Detached HEAD"),
        ("permission denied", "Permission denied"),
        ("docker daemon", "Docker daemon"),
    ] {
        let output = lbc()
            .args(["search", query])
            .output()
            .expect("lbc search should run");
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains(expected),
            "searching {query:?} should surface {expected:?}: {stdout}"
        );
    }
}

#[test]
fn knowledge_install_list_remove_roundtrip() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let home = std::env::temp_dir().join(format!("lbc-cli-pkg-{nonce}"));
    let data_home = home.join("data");
    let package = home.join("team-rules");
    std::fs::create_dir_all(&package).expect("package directory should be creatable");
    std::fs::write(
        package.join("package.toml"),
        "name = \"team-rules\"\nversion = \"1.0.0\"\ndescription = \"Team conventions\"\n",
    )
    .expect("manifest should be writable");
    std::fs::write(
        package.join("rule.md"),
        "---\nid: team-format\ntitle: Team formatting rule\n---\n# Team formatting rule\n\nAlways run cargo fmt.\n",
    )
    .expect("document should be writable");

    let install = lbc()
        .args(["knowledge", "install"])
        .arg(&package)
        .env("XDG_DATA_HOME", &data_home)
        .output()
        .expect("lbc knowledge install should run");
    assert!(
        install.status.success(),
        "install failed: {}",
        String::from_utf8_lossy(&install.stderr)
    );
    let stdout = String::from_utf8_lossy(&install.stdout);
    assert!(stdout.contains("team-rules"));
    assert!(stdout.contains("Documents:"));

    let list = lbc()
        .args(["knowledge", "list"])
        .env("XDG_DATA_HOME", &data_home)
        .output()
        .expect("lbc knowledge list should run");
    assert!(list.status.success());
    let stdout = String::from_utf8_lossy(&list.stdout);
    assert!(stdout.contains("team-rules 1.0.0"));
    assert!(stdout.contains("Team conventions"));

    let search = lbc()
        .args(["search", "team formatting"])
        .env("XDG_DATA_HOME", &data_home)
        .output()
        .expect("lbc search should see installed packages");
    assert!(search.status.success());
    let stdout = String::from_utf8_lossy(&search.stdout);
    assert!(
        stdout.contains("Team formatting rule"),
        "installed doc should be searchable: {stdout}"
    );

    let remove = lbc()
        .args(["knowledge", "remove", "team-rules"])
        .env("XDG_DATA_HOME", &data_home)
        .output()
        .expect("lbc knowledge remove should run");
    assert!(
        remove.status.success(),
        "remove failed: {}",
        String::from_utf8_lossy(&remove.stderr)
    );
    let list = lbc()
        .args(["knowledge", "list"])
        .env("XDG_DATA_HOME", &data_home)
        .output()
        .expect("lbc knowledge list should run");
    assert!(String::from_utf8_lossy(&list.stdout).contains("No knowledge packages installed"));

    std::fs::remove_dir_all(home).expect("temporary directories should be removable");
}

#[test]
fn knowledge_install_rejects_an_invalid_package() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let home = std::env::temp_dir().join(format!("lbc-cli-badpkg-{nonce}"));
    let package = home.join("bad");
    std::fs::create_dir_all(&package).expect("package directory should be creatable");
    std::fs::write(
        package.join("package.toml"),
        "name = \"BAD NAME\"\nversion = \"1.0.0\"\n",
    )
    .expect("manifest should be writable");

    let output = lbc()
        .args(["knowledge", "install"])
        .arg(&package)
        .env("XDG_DATA_HOME", home.join("data"))
        .output()
        .expect("lbc knowledge install should run");
    assert!(
        !output.status.success(),
        "invalid packages must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid"),
        "stderr should explain: {stderr}"
    );
    std::fs::remove_dir_all(home).expect("temporary directories should be removable");
}

#[test]
fn add_list_inspect_ask_edit_roundtrip_uses_current_content() {
    let home = temporary_home("entry-roundtrip");
    std::fs::create_dir_all(&home).expect("fixture root");
    let first = home.join("first.md");
    let second = home.join("second.md");
    std::fs::write(
        &first,
        "Change the demo service development port to 4318, then confirm it listens on 4318.\n",
    )
    .unwrap();
    std::fs::write(
        &second,
        "Change the demo service development port to 4429, then confirm it listens on 4429.\n",
    )
    .unwrap();

    let added = isolated_lbc(&home)
        .args([
            "add",
            "--id",
            "demo-port",
            "--title",
            "Demo service port conflict",
            "--kind",
            "troubleshooting",
            "--file",
        ])
        .arg(&first)
        .output()
        .unwrap();
    assert!(
        added.status.success(),
        "{}",
        String::from_utf8_lossy(&added.stderr)
    );
    assert!(String::from_utf8_lossy(&added.stdout).contains("user:demo-port"));

    let listed = isolated_lbc(&home)
        .args(["list", "--json"])
        .output()
        .unwrap();
    let entries: serde_json::Value = serde_json::from_slice(&listed.stdout).unwrap();
    assert!(
        entries
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["source_id"] == "user:demo-port")
    );

    let inspected = isolated_lbc(&home)
        .args(["inspect", "user:demo-port"])
        .output()
        .unwrap();
    assert!(inspected.status.success());
    assert!(String::from_utf8_lossy(&inspected.stdout).contains("4318"));

    let empty = home.join("empty.md");
    std::fs::write(&empty, "\n").unwrap();
    let rejected = isolated_lbc(&home)
        .args(["edit", "user:demo-port", "--file"])
        .arg(&empty)
        .output()
        .unwrap();
    assert!(!rejected.status.success());
    let preserved = isolated_lbc(&home)
        .args(["inspect", "user:demo-port"])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&preserved.stdout).contains("4318"));

    let first_answer = isolated_lbc(&home)
        .args([
            "ask",
            "How do I resolve the demo service port conflict?",
            "--project",
        ])
        .arg(&home)
        .output()
        .unwrap();
    let first_stdout = String::from_utf8_lossy(&first_answer.stdout);
    assert!(
        first_stdout.contains("4318")
            && first_stdout.contains("user:demo-port")
            && first_stdout.contains("not verified"),
        "{first_stdout}"
    );

    let edited = isolated_lbc(&home)
        .args(["edit", "user:demo-port", "--file"])
        .arg(&second)
        .output()
        .unwrap();
    assert!(
        edited.status.success(),
        "{}",
        String::from_utf8_lossy(&edited.stderr)
    );
    let second_answer = isolated_lbc(&home)
        .args([
            "ask",
            "How do I resolve the demo service port conflict?",
            "--project",
        ])
        .arg(&home)
        .output()
        .unwrap();
    let second_stdout = String::from_utf8_lossy(&second_answer.stdout);
    assert!(
        second_stdout.contains("4429") && !second_stdout.contains("4318"),
        "{second_stdout}"
    );
    std::fs::remove_dir_all(home).unwrap();
}

#[test]
fn e0308_is_answered_from_builtin_knowledge_without_a_special_rule() {
    let home = temporary_home("e0308");
    std::fs::create_dir_all(&home).unwrap();
    let mut child = isolated_lbc(&home)
        .arg("explain")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"error[E0308]: mismatched types\n expected String, found &str\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("builtin:rust-e0308"), "{stdout}");
    assert!(
        stdout.contains("expected") && stdout.contains("found") && stdout.contains("to_string"),
        "{stdout}"
    );
    assert!(
        stdout.contains("Retrieved knowledge (unverified)"),
        "{stdout}"
    );
    std::fs::remove_dir_all(home).unwrap();
}

#[test]
fn config_and_doctor_mask_url_credentials_without_modifying_config() {
    let home = tempfile::tempdir().unwrap();
    let path = home.path().join("config/lbc/config.toml");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let content = "[ai]\nprovider = 'openai-compat'\nmodel = 'mock'\nbase_url = 'https://fixture-user:fixture-password@example.invalid/v1'\n";
    std::fs::write(&path, content).unwrap();
    for args in [
        vec!["config", "show"],
        vec!["config", "show", "--json"],
        vec!["doctor", "--json"],
    ] {
        let result = isolated_lbc(home.path()).args(args).output().unwrap();
        assert!(result.status.success());
        let output = String::from_utf8_lossy(&result.stdout);
        assert!(!output.contains("fixture-user") && !output.contains("fixture-password"));
        assert!(output.contains("[REDACTED]"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), content);
    }
}

#[test]
fn builtin_can_be_inspected_and_explicitly_overridden() {
    let home = temporary_home("override");
    std::fs::create_dir_all(&home).unwrap();
    let replacement = home.join("replacement.md");
    std::fs::write(
        &replacement,
        "Use the unique BLUE-PINE type conversion procedure.\n",
    )
    .unwrap();
    let inspect = isolated_lbc(&home)
        .args(["inspect", "builtin:rust-e0308"])
        .output()
        .unwrap();
    assert!(inspect.status.success());
    assert!(String::from_utf8_lossy(&inspect.stdout).contains("Mismatched types"));
    let edit = isolated_lbc(&home)
        .args(["edit", "builtin:rust-e0308", "--override", "--file"])
        .arg(&replacement)
        .output()
        .unwrap();
    assert!(
        edit.status.success(),
        "{}",
        String::from_utf8_lossy(&edit.stderr)
    );
    let answer = isolated_lbc(&home)
        .args(["ask", "E0308 mismatched types", "--project"])
        .arg(&home)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&answer.stdout);
    assert!(
        stdout.contains("BLUE-PINE") && stdout.contains("user:rust-e0308"),
        "{stdout}"
    );
    assert!(
        !stdout.contains("passing `&str`"),
        "builtin should not remain effective: {stdout}"
    );
    let inspect = isolated_lbc(&home)
        .args(["inspect", "user:rust-e0308", "--json"])
        .output()
        .unwrap();
    let document: serde_json::Value = serde_json::from_slice(&inspect.stdout).unwrap();
    assert_eq!(document["metadata"]["error_code"], "E0308");
    assert_eq!(document["verification_status"], "unverified");
    std::fs::remove_dir_all(home).unwrap();
}

#[test]
fn editing_changed_content_invalidates_previous_verification() {
    let home = tempfile::tempdir().unwrap();
    let notes = home.path().join("data/lbc/notes");
    std::fs::create_dir_all(&notes).unwrap();
    std::fs::write(notes.join("checked.md"), "---\nid: checked\ntitle: Checked note\nverification_status: user-reported\nerror_code: DEMO42\ntags: [demo]\n---\nPreviously tested instructions.\n").unwrap();
    let replacement = home.path().join("replacement.md");
    std::fs::write(
        &replacement,
        "Different instructions without a recorded test.\n",
    )
    .unwrap();
    let edit = isolated_lbc(home.path())
        .args(["edit", "user:checked", "--file"])
        .arg(&replacement)
        .output()
        .unwrap();
    assert!(
        edit.status.success(),
        "{}",
        String::from_utf8_lossy(&edit.stderr)
    );
    let inspect = isolated_lbc(home.path())
        .args(["inspect", "user:checked", "--json"])
        .output()
        .unwrap();
    let document: serde_json::Value = serde_json::from_slice(&inspect.stdout).unwrap();
    assert_eq!(document["verification_status"], "unverified");
    assert_eq!(document["metadata"]["error_code"], "DEMO42");
    assert_eq!(document["metadata"]["tags"][0], "demo");
    assert_eq!(
        std::fs::read_dir(notes).unwrap().count(),
        1,
        "temporary file leaked"
    );
}

#[cfg(unix)]
#[test]
fn quoted_editor_uses_existing_override_and_failed_launch_cleans_up() {
    let home = tempfile::tempdir().unwrap();
    let replacement = home.path().join("replacement.md");
    let body = "Retain this distinctive existing override body.\n";
    std::fs::write(&replacement, body).unwrap();
    let edit = isolated_lbc(home.path())
        .args(["edit", "builtin:rust-e0308", "--override", "--file"])
        .arg(&replacement)
        .output()
        .unwrap();
    assert!(edit.status.success());
    let editor = home.path().join("editor with spaces.sh");
    std::fs::write(&editor, "test \"$1\" = 'argument with spaces' || exit 4\ncase \"$(head -n 1 \"$2\")\" in 'Retain this distinctive existing override body.') exit 0;; *) exit 5;; esac\n").unwrap();
    let temps = home.path().join("editor-temporary");
    std::fs::create_dir(&temps).unwrap();
    let edit = isolated_lbc(home.path())
        .args(["edit", "builtin:rust-e0308", "--override"])
        .env(
            "VISUAL",
            format!("/bin/sh '{}' 'argument with spaces'", editor.display()),
        )
        .env("TMPDIR", &temps)
        .output()
        .unwrap();
    assert!(
        edit.status.success(),
        "{}",
        String::from_utf8_lossy(&edit.stderr)
    );
    assert_eq!(std::fs::read_dir(&temps).unwrap().count(), 0);
    let path = home.path().join("data/lbc/notes/rust-e0308.md");
    let before = std::fs::read(&path).unwrap();
    for command in ["/no/such/lbc-editor", "/bin/false"] {
        let edit = isolated_lbc(home.path())
            .args(["edit", "user:rust-e0308"])
            .env("VISUAL", command)
            .env("TMPDIR", &temps)
            .output()
            .unwrap();
        assert!(!edit.status.success());
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert_eq!(std::fs::read_dir(&temps).unwrap().count(), 0);
    }
}

#[test]
fn unhealthy_doctor_emits_json_and_fails() {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join("config/lbc")).unwrap();
    std::fs::write(
        home.path().join("config/lbc/config.toml"),
        "[ai]\nprovider = 'openrouter'\nmodel = 'test-model'\n",
    )
    .unwrap();
    let result = isolated_lbc(home.path())
        .args(["doctor", "--json"])
        .current_dir(home.path())
        .output()
        .unwrap();
    assert!(!result.status.success());
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["healthy"], false);
    assert!(
        report["checks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|check| check["name"] == "AI provider" && check["ok"] == false)
    );
}

#[test]
fn default_chat_does_not_create_history_and_persistent_chat_does() {
    let home = temporary_home("history");
    std::fs::create_dir_all(home.join("config/lbc")).unwrap();
    let mut child = isolated_lbc(&home)
        .arg("chat")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"ownership\n/exit\n")
        .unwrap();
    assert!(child.wait().unwrap().success());
    assert!(!home.join("data/lbc/history/default.json").exists());

    std::fs::write(
        home.join("config/lbc/config.toml"),
        "[memory]\nmode = \"persistent\"\n",
    )
    .unwrap();
    let mut child = isolated_lbc(&home)
        .arg("chat")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"ownership API_KEY=history-secret-value\n/exit\n")
        .unwrap();
    assert!(child.wait().unwrap().success());
    assert!(home.join("data/lbc/history/default.json").exists());
    let saved = std::fs::read_to_string(home.join("data/lbc/history/default.json")).unwrap();
    assert!(!saved.contains("history-secret-value"));
    assert!(saved.contains("[REDACTED]"));
    let mut restored = isolated_lbc(&home)
        .arg("chat")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    restored
        .stdin
        .take()
        .unwrap()
        .write_all(b"second-session-unique\n/exit\n")
        .unwrap();
    assert!(restored.wait().unwrap().success());
    let saved = std::fs::read_to_string(home.join("data/lbc/history/default.json")).unwrap();
    assert!(
        saved.contains("User: ownership"),
        "previous session was not restored"
    );
    assert!(saved.contains("second-session-unique"));
    assert!(!saved.contains("history-secret-value"));
    let cleared = isolated_lbc(&home)
        .args(["history", "clear", "--json"])
        .output()
        .unwrap();
    assert!(cleared.status.success());
    assert!(!home.join("data/lbc/history/default.json").exists());
    std::fs::remove_dir_all(home).unwrap();
}

#[test]
fn timed_out_provider_preserves_the_offline_answer() {
    let home = tempfile::tempdir().unwrap();
    let listener =
        TcpListener::bind("127.0.0.1:0").expect("loopback sockets required for timeout regression");
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let (stop, stopped) = mpsc::channel();
    let server = thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "provider received no connection"
                    );
                    thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(error) => panic!("accept failed: {error}"),
            }
        };
        // Deliberately send no HTTP headers. Release the connection after the
        // client's real 45-second deadline rather than simulating an error.
        let _ = stopped.recv_timeout(std::time::Duration::from_secs(55));
        drop(stream);
    });
    std::fs::create_dir_all(home.path().join("config/lbc")).unwrap();
    std::fs::write(
        home.path().join("config/lbc/config.toml"),
        format!(
            "[ai]\nprovider = 'openai-compat'\nmodel = 'mock'\nbase_url = 'http://{address}/v1'\n"
        ),
    )
    .unwrap();
    let result = isolated_lbc(home.path())
        .args(["ask", "E0308 mismatched types", "--ai", "--json"])
        .current_dir(home.path())
        .output()
        .unwrap();
    let _ = stop.send(());
    server.join().unwrap();
    assert!(result.status.success());
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert!(report.get("ai").is_none());
    assert!(
        report["ai_error"].as_str().unwrap().contains("timed out"),
        "{report}"
    );
    assert!(
        report["offline_answer"]
            .as_str()
            .unwrap()
            .contains("builtin:rust-e0308")
    );
    assert!(
        report["offline_answer"]
            .as_str()
            .unwrap()
            .contains("Mismatched types")
    );
}

#[test]
fn offline_commands_never_connect_to_the_configured_provider() {
    let home = tempfile::tempdir().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0")
        .expect("loopback sockets required for the privacy regression test");
    listener.set_nonblocking(true).unwrap();
    std::fs::create_dir_all(home.path().join("config/lbc")).unwrap();
    std::fs::write(
        home.path().join("config/lbc/config.toml"),
        format!(
            "[ai]\nprovider = 'openai-compat'\nmodel = 'mock'\nbase_url = 'http://{}/v1'\n",
            listener.local_addr().unwrap()
        ),
    )
    .unwrap();
    let result = isolated_lbc(home.path())
        .args(["ask", "E0308", "--json"])
        .current_dir(home.path())
        .output()
        .unwrap();
    assert!(result.status.success());
    for (command, input) in [
        ("explain", "error[E0308]: mismatched types\n"),
        ("chat", "E0308\n/exit\n"),
    ] {
        let mut child = isolated_lbc(home.path())
            .arg(command)
            .current_dir(home.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        let result = child.wait_with_output().unwrap();
        assert!(result.status.success(), "offline {command} failed");
    }
    match listener.accept() {
        Err(error) => assert_eq!(error.kind(), std::io::ErrorKind::WouldBlock),
        Ok(_) => panic!("a command contacted the provider without --ai"),
    }
    assert!(!home.path().join("data/lbc/history").exists());
}

#[test]
fn mock_ai_receives_retrieved_passage_and_redacts_secrets() {
    let home = temporary_home("mock-ai");
    std::fs::create_dir_all(home.join("config/lbc")).unwrap();
    let note = home.join("note.md");
    std::fs::write(
        &note,
        "Use the unique ORANGE-CEDAR recovery procedure.\n\nAPI_KEY=never-send-this-secret\n",
    )
    .unwrap();
    let added = isolated_lbc(&home)
        .args([
            "add",
            "--id",
            "orange-cedar",
            "--title",
            "Orange cedar recovery",
            "--file",
        ])
        .arg(&note)
        .output()
        .unwrap();
    assert!(added.status.success());

    let listener = TcpListener::bind("127.0.0.1:0").expect(
        "mock provider requires loopback sockets; rerun outside a socket-restricted sandbox",
    );
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let (sender, receiver) = mpsc::channel();
    let server = thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let (mut stream, _) = loop {
            match listener.accept() {
                Ok(connection) => break connection,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "mock provider received no request"
                    );
                    thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(error) => panic!("mock provider accept failed: {error}"),
            }
        };
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut request = Vec::new();
        let mut buffer = [0_u8; 4096];
        let mut expected = None;
        loop {
            let count = stream.read(&mut buffer).unwrap();
            if count == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..count]);
            if expected.is_none()
                && let Some(header_end) = request.windows(4).position(|part| part == b"\r\n\r\n")
            {
                let headers = String::from_utf8_lossy(&request[..header_end]);
                let content_length = headers.lines().find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .and_then(|value| value.trim().parse::<usize>().ok())
                });
                expected = content_length.map(|length| header_end + 4 + length);
            }
            if expected.is_some_and(|length| request.len() >= length) {
                break;
            }
        }
        sender
            .send(String::from_utf8_lossy(&request).into_owned())
            .unwrap();
        let body = r#"{"model":"mock-model","choices":[{"message":{"content":"Grounded response.\n\nConfidence: medium"}}]}"#;
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .unwrap();
    });
    std::fs::write(
        home.join("config/lbc/config.toml"),
        format!(
            "[ai]\nprovider = \"openai-compat\"\nmodel = \"mock-model\"\nbase_url = \"http://{address}/v1\"\n"
        ),
    )
    .unwrap();
    let output = isolated_lbc(&home)
        .args(["ask", "How does orange cedar recovery work?", "--project"])
        .arg(&home)
        .args(["--ai", "--json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    server.join().unwrap();
    let request = receiver.recv().unwrap();
    assert!(request.contains("ORANGE-CEDAR"));
    assert!(request.contains("user:orange-cedar"));
    assert!(!request.contains("never-send-this-secret"));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["ai"]["analysis"], "Grounded response.");
    std::fs::remove_dir_all(home).unwrap();
}

#[test]
fn thai_note_produces_meaningful_thai_answer_with_stable_commands() {
    let home = temporary_home("thai");
    std::fs::create_dir_all(home.join("config/lbc")).unwrap();
    std::fs::write(
        home.join("config/lbc/config.toml"),
        "[output]\nlanguage = \"th\"\n",
    )
    .unwrap();
    let note = home.join("thai.md");
    std::fs::write(
        &note,
        "เมื่อพอร์ตชนกัน ให้เปลี่ยนพอร์ตของบริการเป็น 4318 แล้วใช้ `cargo check` เพื่อตรวจสอบ\n",
    )
    .unwrap();
    let added = isolated_lbc(&home)
        .args([
            "add",
            "--id",
            "thai-port",
            "--title",
            "วิธีแก้ปัญหาพอร์ตชนกัน",
            "--file",
        ])
        .arg(&note)
        .output()
        .unwrap();
    assert!(added.status.success());
    let answer = isolated_lbc(&home)
        .args(["ask", "แก้ปัญหาพอร์ตชนกันอย่างไร", "--project"])
        .arg(&home)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&answer.stdout);
    assert!(stdout.contains("คำแนะนำจากความรู้ที่ค้นพบ"), "{stdout}");
    assert!(
        stdout.contains("เปลี่ยนพอร์ต") && stdout.contains("4318"),
        "{stdout}"
    );
    assert!(
        stdout.contains("cargo check") && stdout.contains("user:thai-port"),
        "{stdout}"
    );
    std::fs::remove_dir_all(home).unwrap();
}

#[test]
fn explain_records_bounded_source_lines_and_multiple_diagnostics() {
    let home = temporary_home("evidence");
    std::fs::create_dir_all(home.join("src")).unwrap();
    std::fs::write(
        home.join("Cargo.toml"),
        "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::write(
        home.join("src/main.rs"),
        "fn one() {}\nfn two() {}\nfn three() {}\nfn target() { let x: String = \"x\"; }\nfn five() {}\n",
    )
    .unwrap();
    let input = b"error[E0308]: mismatched types\n --> src/main.rs:4:31\nerror[E0382]: use of moved value\n --> src/main.rs:5:1\n";
    let mut child = isolated_lbc(&home)
        .args(["explain", "--stdin", "--json", "--project"])
        .arg(&home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["diagnostics_detected"], 2);
    let evidence = report["project_evidence"].as_array().unwrap();
    let source = evidence
        .iter()
        .find(|item| item["path"] == "src/main.rs")
        .unwrap();
    assert_eq!(source["start_line"], 1);
    assert_eq!(source["end_line"], 5);
    assert!(source["content"].as_str().unwrap().contains("4: fn target"));
    assert!(
        report["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str().unwrap().contains("2 diagnostics"))
    );
    std::fs::remove_dir_all(home).unwrap();
}

#[test]
fn package_collision_is_ambiguous_and_does_not_shadow_builtin() {
    let home = temporary_home("collision");
    let package = home.join("collision-package");
    std::fs::create_dir_all(&package).unwrap();
    std::fs::write(
        package.join("package.toml"),
        "name = \"collision\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    std::fs::write(
        package.join("doc.md"),
        "---\nid: rust-e0308\ntitle: Package collision\nerror_code: E0308\n---\nPACKAGE-COLLISION-CONTENT\n",
    )
    .unwrap();
    let installed = isolated_lbc(&home)
        .args(["knowledge", "install"])
        .arg(&package)
        .output()
        .unwrap();
    assert!(installed.status.success());
    let ambiguous = isolated_lbc(&home)
        .args(["inspect", "rust-e0308", "--project"])
        .arg(&home)
        .output()
        .unwrap();
    assert!(!ambiguous.status.success());
    let stderr = String::from_utf8_lossy(&ambiguous.stderr);
    assert!(stderr.contains("builtin:rust-e0308"), "{stderr}");
    assert!(stderr.contains("package:collision:rust-e0308"), "{stderr}");
    let builtin = isolated_lbc(&home)
        .args(["inspect", "builtin:rust-e0308", "--project"])
        .arg(&home)
        .output()
        .unwrap();
    assert!(builtin.status.success());
    assert!(!String::from_utf8_lossy(&builtin.stdout).contains("PACKAGE-COLLISION-CONTENT"));
    std::fs::remove_dir_all(home).unwrap();
}

#[test]
fn index_reports_invalid_documents_while_valid_notes_remain_searchable() {
    let home = temporary_home("invalid-index");
    let notes = home.join("data/lbc/notes");
    std::fs::create_dir_all(&notes).unwrap();
    std::fs::write(notes.join("invalid.md"), "missing frontmatter\n").unwrap();
    std::fs::write(
        notes.join("valid.md"),
        "---\nid: valid-note\ntitle: Valid nebula note\n---\nUNIQUE-NEBULA-GUIDANCE\n",
    )
    .unwrap();
    let indexed = isolated_lbc(&home)
        .args(["index", "--project"])
        .arg(&home)
        .arg("--json")
        .output()
        .unwrap();
    assert!(!indexed.status.success());
    let report: serde_json::Value = serde_json::from_slice(&indexed.stdout).unwrap();
    assert_eq!(report["invalid_documents"], 1);
    assert!(
        report["invalid"][0]["path"]
            .as_str()
            .unwrap()
            .ends_with("invalid.md")
    );
    let search = isolated_lbc(&home)
        .args(["search", "nebula guidance", "--project"])
        .arg(&home)
        .output()
        .unwrap();
    assert!(search.status.success());
    assert!(String::from_utf8_lossy(&search.stdout).contains("UNIQUE-NEBULA-GUIDANCE"));
    std::fs::remove_dir_all(home).unwrap();
}

#[test]
fn empty_and_oversized_questions_fail_with_actionable_errors() {
    let home = temporary_home("query-limits");
    std::fs::create_dir_all(&home).unwrap();
    for query in [String::new(), "x".repeat(9 * 1024)] {
        for command in ["ask", "search"] {
            let output = isolated_lbc(&home)
                .args([command, &query])
                .current_dir(&home)
                .output()
                .unwrap();
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(error.contains("empty") || error.contains("8 KB"));
        }
    }
    let mut child = isolated_lbc(&home)
        .arg("chat")
        .current_dir(&home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all("x".repeat(9 * 1024).as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("8 KB"));
    assert!(!home.join("data/lbc/history/default.json").exists());
    std::fs::remove_dir_all(home).unwrap();
}

#[test]
fn offline_ask_json_does_not_echo_question_credentials() {
    let home = temporary_home("private-question");
    std::fs::create_dir_all(&home).unwrap();
    let output = isolated_lbc(&home)
        .args(["ask", "API_KEY=fixture-question-secret", "--json"])
        .current_dir(&home)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("fixture-question-secret"));
    let _: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    std::fs::remove_dir_all(home).unwrap();
}
