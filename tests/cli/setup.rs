use super::*;
use std::path::Path;

fn run(home: &Path, args: &[&str]) -> std::process::Output {
    isolated_lbc(home).args(args).output().unwrap()
}

fn success(home: &Path, args: &[&str]) -> serde_json::Value {
    let output = run(home, args);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn interactive(home: &Path, input: &str) -> std::process::Output {
    let mut child = isolated_lbc(home)
        .arg("setup")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn fresh_setup_supports_the_offline_knowledge_loop() {
    let home = tempfile::tempdir().unwrap();
    std::fs::write(
        home.path().join("Cargo.toml"),
        "[package]\nname = 'fixture'\nversion = '0.1.0'\n",
    )
    .unwrap();
    let report = success(home.path(), &["setup", "--yes", "--json"]);
    assert_eq!(report["provider"], "off");
    assert!(report["stack"].as_str().unwrap().contains("Rust"));
    assert_eq!(report["connectivity"], "untested; no network requests made");
    assert!(home.path().join(".lbc/knowledge").is_dir());
    assert!(home.path().join("data/lbc/notes").is_dir());
    let config: toml::Value = std::fs::read_to_string(home.path().join("config/lbc/config.toml"))
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(config["ai"]["provider"].as_str(), Some("off"));
    success(
        home.path(),
        &[
            "learn",
            "--yes",
            "--problem",
            "Rust E0308 setup incident",
            "--solution",
            "Borrow the SETUP-WORKAROUND value",
            "--json",
        ],
    );
    assert_eq!(
        success(home.path(), &["search", "SETUP-WORKAROUND", "--json"])[0]["source_id"],
        "project:rust-e0308-setup-incident"
    );
    std::fs::write(
        home.path().join("error.log"),
        "error[E0308]: mismatched types\n",
    )
    .unwrap();
    assert!(
        success(home.path(), &["explain", "error.log", "--json"])["knowledge"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["source_id"] == "project:rust-e0308-setup-incident")
    );
}

#[test]
fn setup_preserves_existing_configuration_unknown_keys_and_knowledge() {
    let home = tempfile::tempdir().unwrap();
    let path = home.path().join("config/lbc/config.toml");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "custom = 'keep'\n[output]\nlanguage = 'th'\n[scanner]\nmax_file_size_kb = 128\n[ai]\nprovider = 'openai'\nmodel = 'my-model'\nfuture = 'preserved'\n[memory]\nmode = 'persistent'\n").unwrap();
    std::fs::create_dir_all(home.path().join(".lbc/knowledge")).unwrap();
    let note = home.path().join(".lbc/knowledge/existing.md");
    std::fs::write(&note, "existing bytes").unwrap();
    for _ in 0..2 {
        success(home.path(), &["setup", "--yes", "--json"]);
    }
    let config: toml::Value = std::fs::read_to_string(&path).unwrap().parse().unwrap();
    assert_eq!(config["custom"].as_str(), Some("keep"));
    assert_eq!(config["ai"]["future"].as_str(), Some("preserved"));
    assert_eq!(config["ai"]["model"].as_str(), Some("my-model"));
    assert_eq!(config["ai"]["provider"].as_str(), Some("openai"));
    assert_eq!(config["output"]["language"].as_str(), Some("th"));
    assert_eq!(
        config["scanner"]["max_file_size_kb"].as_integer(),
        Some(128)
    );
    assert_eq!(config["memory"]["mode"].as_str(), Some("persistent"));
    assert_eq!(std::fs::read_to_string(&note).unwrap(), "existing bytes");
    let output = interactive(home.path(), "\n\n\n\n");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let updated: toml::Value = std::fs::read_to_string(&path).unwrap().parse().unwrap();
    assert_eq!(updated, config);
}

#[test]
fn setup_ai_is_environment_only_and_never_contacts_the_provider() {
    let home = tempfile::tempdir().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    let output = isolated_lbc(home.path())
        .args([
            "setup",
            "--yes",
            "--provider",
            "openai",
            "--base-url",
            &url,
            "--json",
        ])
        .env("OPENAI_API_KEY", "sk-private-setup-secret-123456789")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["environment_variables"][0], "OPENAI_API_KEY");
    assert_eq!(report["credentials"], "detected; validity untested");
    let stored = std::fs::read_to_string(home.path().join("config/lbc/config.toml")).unwrap();
    for bytes in [&output.stdout[..], &output.stderr[..], stored.as_bytes()] {
        assert!(!String::from_utf8_lossy(bytes).contains("private-setup-secret"));
    }
    assert!(
        matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock)
    );
    let report = success(home.path(), &["setup", "--yes", "--json"]);
    assert_eq!(
        report["credentials"],
        "missing; offline knowledge remains available"
    );
}

#[test]
fn invalid_cancelled_and_incomplete_setup_never_mutates_stores() {
    for input in ["", "\n", "\nn\n", "invalid\n", "\nmaybe\n"] {
        let home = tempfile::tempdir().unwrap();
        let output = interactive(home.path(), input);
        assert_eq!(output.status.success(), input == "\nn\n");
        assert!(!home.path().join("config/lbc/config.toml").exists());
        assert!(!home.path().join(".lbc").exists());
        assert!(!home.path().join("data/lbc/notes").exists());
    }
    for args in [
        vec!["setup", "--yes", "--provider", "unknown"],
        vec!["setup", "--json"],
        vec!["setup", "--yes", "--provider", "openai-compat"],
        vec!["setup", "--yes", "--provider", "openrouter"],
        vec!["setup", "--yes", "--project", "missing"],
    ] {
        let home = tempfile::tempdir().unwrap();
        assert!(!run(home.path(), &args).status.success());
        assert!(!home.path().join("config/lbc/config.toml").exists());
        assert!(!home.path().join(".lbc").exists());
        assert!(!home.path().join("data/lbc/notes").exists());
    }
}

#[test]
fn setup_rejects_secret_and_invalid_ai_inputs_without_echoing_or_storing_them() {
    for (flag, value) in [
        ("--model", "sk-private-setup-secret-123456789"),
        ("--model", "API_KEY=fixture-setup-secret"),
        (
            "--model",
            "https://user:fixture-setup-secret@example.invalid/model",
        ),
        (
            "--base-url",
            "https://user:fixture-setup-secret@example.invalid/v1",
        ),
        (
            "--base-url",
            "https://example.invalid/v1?api_key=fixture-setup-secret",
        ),
        (
            "--base-url",
            "https://example.invalid/v1?credential=fixture-setup-secret",
        ),
        ("--base-url", "not-a-url"),
    ] {
        let home = tempfile::tempdir().unwrap();
        let output = run(
            home.path(),
            &["setup", "--yes", "--provider", "openai", flag, value],
        );
        assert!(!output.status.success(), "accepted {flag}");
        assert!(!String::from_utf8_lossy(&output.stdout).contains("setup-secret"));
        assert!(!String::from_utf8_lossy(&output.stderr).contains("setup-secret"));
        assert!(!home.path().join("config/lbc/config.toml").exists());
        assert!(!home.path().join(".lbc").exists());
    }
}

#[cfg(unix)]
#[test]
fn setup_rejects_unsafe_paths_before_writing_anything() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    for target in [".lbc", "data/lbc/notes", "config/lbc/config.toml"] {
        let home = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let path = home.path().join(target);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        symlink(outside.path().join("absent"), &path).unwrap();
        let output = run(home.path(), &["setup", "--yes"]);
        assert!(!output.status.success(), "accepted {target}");
        assert!(std::fs::read_dir(outside.path()).unwrap().next().is_none());
        if target != "config/lbc/config.toml" {
            assert!(!home.path().join("config/lbc/config.toml").exists());
        }
    }
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir(home.path().join(".lbc")).unwrap();
    std::fs::set_permissions(
        home.path().join(".lbc"),
        std::fs::Permissions::from_mode(0o777),
    )
    .unwrap();
    assert!(!run(home.path(), &["setup", "--yes"]).status.success());
    assert!(!home.path().join("config/lbc/config.toml").exists());
}

#[test]
fn setup_uses_explicit_project_scope_and_supports_existing_providers() {
    for provider in [
        "off",
        "openai",
        "zai",
        "glm",
        "ollama",
        "openrouter",
        "openai-compat",
    ] {
        let home = tempfile::tempdir().unwrap();
        let project = home.path().join("nested");
        std::fs::create_dir(&project).unwrap();
        std::fs::write(home.path().join("Cargo.toml"), "[package]").unwrap();
        let report = success(
            home.path(),
            &[
                "setup",
                "--yes",
                "--project",
                "nested",
                "--provider",
                provider,
                "--model",
                "fixture-model",
                "--base-url",
                "http://127.0.0.1:9/v1",
                "--json",
            ],
        );
        assert_eq!(report["provider"], provider);
        assert_eq!(report["stack"], "Unknown project");
        assert_eq!(
            Path::new(report["project"].as_str().unwrap()),
            project.canonicalize().unwrap()
        );
        assert!(project.join(".lbc/knowledge").is_dir());
        assert!(!home.path().join(".lbc").exists());
    }
}

#[test]
fn interactive_setup_defaults_offline_and_reports_destinations() {
    let home = tempfile::tempdir().unwrap();
    let output = interactive(home.path(), "\n\n");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let prompts = String::from_utf8_lossy(&output.stderr);
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(prompts.contains("AI provider:"));
    assert!(prompts.contains("Save configuration and initialize knowledge stores?"));
    for expected in [
        "OS:",
        "Project:",
        "Detected stack:",
        "Config:",
        "Project knowledge:",
        "Personal knowledge:",
        "AI provider: off",
    ] {
        assert!(text.contains(expected), "missing {expected}");
    }
    assert!(home.path().join(".lbc/knowledge").is_dir());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(home.path().join("config/lbc/config.toml"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}

#[test]
fn setup_leaves_invalid_existing_config_and_regular_file_destinations_untouched() {
    for original in ["not valid toml!", "[scanner]\nmax_file_size_kb = 0\n"] {
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join("config/lbc/config.toml");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, original).unwrap();
        assert!(!run(home.path(), &["setup", "--yes"]).status.success());
        assert_eq!(std::fs::read_to_string(path).unwrap(), original);
        assert!(!home.path().join(".lbc").exists());
        assert!(!home.path().join("data/lbc/notes").exists());
    }
    for destination in [".lbc/knowledge", "data/lbc/notes"] {
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join(destination);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "untouched").unwrap();
        assert!(!run(home.path(), &["setup", "--yes"]).status.success());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "untouched");
        assert!(!home.path().join("config/lbc/config.toml").exists());
    }
}

#[test]
fn existing_config_subcommands_keep_working_after_setup() {
    let home = tempfile::tempdir().unwrap();
    success(home.path(), &["setup", "--yes", "--json"]);
    let path = home.path().join("config/lbc/config.toml");
    let original = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, format!("extension = 'keep'\n{original}")).unwrap();
    assert!(
        run(home.path(), &["config", "set", "output.language", "th"])
            .status
            .success()
    );
    let shown = success(home.path(), &["config", "show", "--json"]);
    assert_eq!(shown["output"]["language"], "th");
    let document: toml::Value = std::fs::read_to_string(path).unwrap().parse().unwrap();
    assert_eq!(document["extension"].as_str(), Some("keep"));
}
