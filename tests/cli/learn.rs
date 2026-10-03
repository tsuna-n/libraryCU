use super::*;
use std::path::Path;

fn json(home: &Path, args: &[&str]) -> serde_json::Value {
    let output = isolated_lbc(home).args(args).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn interactive(home: &Path, args: &[&str], input: &str) -> std::process::Output {
    let mut child = isolated_lbc(home)
        .args(args)
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
fn capture_find_reuse_edit_and_external_markdown_work_offline() {
    let home = tempfile::tempdir().unwrap();
    // No model credentials are needed for capture or reuse.
    std::fs::create_dir_all(home.path().join("config/lbc")).unwrap();
    std::fs::write(
        home.path().join("config/lbc/config.toml"),
        "[ai]\nprovider = 'off'\n",
    )
    .unwrap();
    let capture = json(
        home.path(),
        &[
            "learn",
            "--yes",
            "--id",
            "db-borrow",
            "--problem",
            "Rust E0308 String to &str mismatch",
            "--cause",
            "Function expected &str but database value was String",
            "--solution",
            "Borrow the database value using &value instead of cloning it.",
            "--verification",
            "cargo test",
            "--context",
            "backend database lookup",
            "--symptoms",
            "Build fails",
            "--reference",
            "https://example.invalid/incident",
            "--tag",
            "database",
            "--framework",
            "axum",
            "--json",
        ],
    );
    assert_eq!(capture["source_id"], "project:db-borrow");
    assert_eq!(capture["kind"], "troubleshooting");
    assert_eq!(capture["metadata"]["language"], "rust");
    assert_eq!(capture["metadata"]["tool"], "cargo");
    assert_eq!(capture["metadata"]["framework"], "axum");
    assert_eq!(
        json(home.path(), &["search", "axum", "--json"])[0]["source_id"],
        "project:db-borrow"
    );
    assert_eq!(capture["metadata"]["error_code"], "E0308");
    assert_eq!(capture["verification_status"], "recorded-check");
    assert!(capture["metadata"]["created_at_unix"].as_u64().unwrap() > 0);
    assert_eq!(
        capture["metadata"]["created_at_unix"],
        capture["metadata"]["updated_at_unix"]
    );
    let path = home.path().join(".lbc/knowledge/db-borrow.md");
    let original = std::fs::read_to_string(&path).unwrap();
    assert!(original.starts_with("---\n"));
    for section in [
        "Problem",
        "Symptoms",
        "Context",
        "Cause",
        "Solution",
        "Verification",
        "References",
    ] {
        assert!(original.contains(&format!("## {section}")));
    }
    json(
        home.path(),
        &[
            "learn",
            "--yes",
            "--user",
            "--id",
            "generic-borrow",
            "--problem",
            "Rust E0308 generic mismatch",
            "--solution",
            "A generic borrowing suggestion.",
            "--json",
        ],
    );
    let results = json(home.path(), &["search", "Rust E0308", "--json"]);
    assert_eq!(results[0]["source_id"], "project:db-borrow");
    assert_eq!(results[0]["match_reason"], "exact error code");
    assert!(
        results[0]["ranking_reasons"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("source preference: project"))
    );
    assert_eq!(
        json(home.path(), &["search", "project:db-borrow", "--json"])[0]["source_id"],
        "project:db-borrow"
    );
    let answer = json(home.path(), &["ask", "Rust E0308", "--json"]);
    assert_eq!(answer["passages"][0]["source_id"], "project:db-borrow");
    assert!(
        answer["offline_answer"]
            .as_str()
            .unwrap()
            .contains("Borrow the database value")
    );
    assert!(
        answer["offline_answer"]
            .as_str()
            .unwrap()
            .contains("not rerun")
    );
    let details = &answer["passages"][0]["details"];
    assert_eq!(details["recorded_verification"], "cargo test");
    let inspect = json(home.path(), &["inspect", "project:db-borrow", "--json"]);
    assert!(
        inspect["body"]
            .as_str()
            .unwrap()
            .contains("backend database lookup")
    );
    std::fs::write(
        home.path().join("build.log"),
        "error[E0308]: mismatched types\n",
    )
    .unwrap();
    let explain = json(home.path(), &["explain", "build.log", "--json"]);
    assert_eq!(
        explain["knowledge"][0]["source_id"], "project:db-borrow",
        "{explain:#}"
    );
    assert_eq!(
        explain["knowledge"][0]["details"]["recorded_verification"],
        "cargo test"
    );
    let rendered = isolated_lbc(home.path())
        .args(["explain", "build.log"])
        .output()
        .unwrap();
    let text = String::from_utf8(rendered.stdout).unwrap();
    for expected in [
        "previous solution from this project",
        "exact error code",
        "same language",
        "Previous solution",
        "Borrow the database value",
        "Verification recorded (not rerun)",
    ] {
        assert!(text.contains(expected), "missing {expected}: {text}");
    }
    // The next command must observe an external editor without an index step.
    std::fs::write(
        &path,
        original.replace(
            "instead of cloning it.",
            "and use the EXTERNAL-UPDATE path.",
        ),
    )
    .unwrap();
    let updated = json(home.path(), &["ask", "Rust E0308", "--json"]);
    assert!(
        updated["offline_answer"]
            .as_str()
            .unwrap()
            .contains("EXTERNAL-UPDATE")
    );
    let replacement = home.path().join("replacement.md");
    std::fs::write(
        &replacement,
        "## Problem\n\nRust E0308\n\n## Solution\n\nUse EDITED-SOLUTION.\n",
    )
    .unwrap();
    let edited = json(
        home.path(),
        &[
            "edit",
            "project:db-borrow",
            "--file",
            "replacement.md",
            "--json",
        ],
    );
    assert_eq!(edited["verification_status"], "unverified");
    assert_eq!(
        edited["metadata"]["created_at_unix"],
        capture["metadata"]["created_at_unix"]
    );
    assert!(
        edited["metadata"]["updated_at_unix"].as_u64().unwrap()
            >= capture["metadata"]["updated_at_unix"].as_u64().unwrap()
    );
    let updated = json(home.path(), &["ask", "Rust E0308", "--json"]);
    assert!(
        updated["offline_answer"]
            .as_str()
            .unwrap()
            .contains("EDITED-SOLUTION")
    );
    assert!(
        updated["passages"][0]["details"]
            .get("recorded_verification")
            .is_none()
    );
    let no_match = json(
        home.path(),
        &["ask", "intergalactic nebula frobnication", "--json"],
    );
    assert_eq!(no_match["answer_status"], "no_adequate_match");
    assert!(no_match["passages"].as_array().unwrap().is_empty());
}

#[test]
fn interactive_capture_preview_confirmation_and_user_destination() {
    let home = tempfile::tempdir().unwrap();
    let output = interactive(
        home.path(),
        &["learn"],
        "Rust E0308 mismatched types\nExpected &str, got String\nBorrow using &value\ncargo test\nuser\n\n",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let prompts = String::from_utf8(output.stderr).unwrap();
    assert!(stdout.contains("Saved user:rust-e0308-mismatched-types"));
    for prompt in [
        "What happened?",
        "What was the cause?",
        "How was it fixed?",
        "How was it verified?",
        "Save to:",
        "## Solution",
        "Save this knowledge? [Y/n]",
    ] {
        assert!(prompts.contains(prompt));
    }
    assert!(!home.path().join(".lbc/knowledge").exists());
    assert!(
        home.path()
            .join("data/lbc/notes/rust-e0308-mismatched-types.md")
            .is_file()
    );
}

#[test]
fn cancellation_eof_and_invalid_inputs_never_publish_a_note() {
    for (input, success) in [
        ("Rust E0308\n\nBorrow &value\n\n\nn\n", true),
        ("Rust E0308\n\nBorrow &value\n\n\n", false),
        ("", false),
        ("Rust E0308\n", false),
        ("Rust E0308\n\nBorrow &value\n\ncloud\n", false),
    ] {
        let home = tempfile::tempdir().unwrap();
        let output = interactive(home.path(), &["learn"], input);
        assert_eq!(output.status.success(), success);
        assert!(!home.path().join(".lbc").exists());
        assert!(!home.path().join("data/lbc/notes").exists());
    }
    let home = tempfile::tempdir().unwrap();
    for args in [
        vec!["learn", "--yes", "--problem", "Rust E0308"],
        vec!["learn", "--yes", "--problem", " ", "--solution", "Borrow"],
        vec![
            "learn",
            "--yes",
            "--problem",
            "Rust E0308",
            "--solution",
            "Borrow",
            "--id",
            "../escape",
        ],
        vec![
            "learn",
            "--yes",
            "--user",
            "--project",
            ".",
            "--problem",
            "Rust E0308",
            "--solution",
            "Borrow",
        ],
    ] {
        assert!(
            !isolated_lbc(home.path())
                .args(args)
                .output()
                .unwrap()
                .status
                .success()
        );
    }
    assert!(!home.path().join(".lbc/knowledge/escape.md").exists());
}

#[test]
fn capture_preserves_collisions_redacts_secrets_and_never_executes_checks() {
    let home = tempfile::tempdir().unwrap();
    let marker = home.path().join("verification-must-not-run");
    let verification = format!("touch {}", marker.display());
    let args = [
        "learn",
        "--yes",
        "--problem",
        "Unique database incident",
        "--solution",
        "password=fixture-secret\nUse the RECOVERY-STEP",
        "--verification",
        &verification,
        "--json",
    ];
    let first = json(home.path(), &args);
    let second = json(home.path(), &args);
    assert_ne!(first["source_id"], second["source_id"]);
    let stored = std::fs::read_to_string(first["path"].as_str().unwrap()).unwrap();
    assert!(!stored.contains("fixture-secret"));
    assert!(stored.contains("RECOVERY-STEP"));
    assert!(!marker.exists());
    let output = isolated_lbc(home.path())
        .args([
            "learn",
            "--yes",
            "--id",
            "unique-database-incident",
            "--problem",
            "Replace?",
            "--solution",
            "must not overwrite",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(
        std::fs::read_to_string(first["path"].as_str().unwrap()).unwrap(),
        stored
    );
    let no_check = json(
        home.path(),
        &[
            "learn",
            "--yes",
            "--user",
            "--problem",
            "Plain note",
            "--solution",
            "Manual workaround",
            "--json",
        ],
    );
    assert_eq!(no_check["verification_status"], "unverified");
    assert!(no_check["body"].as_str().unwrap().contains("## Solution"));
    assert!(
        !no_check["body"]
            .as_str()
            .unwrap()
            .contains("## Verification")
    );
}

#[test]
fn explicit_project_scope_is_preserved_and_other_projects_do_not_see_it() {
    let home = tempfile::tempdir().unwrap();
    let project = home.path().join("backend");
    std::fs::create_dir(&project).unwrap();
    let capture = json(
        home.path(),
        &[
            "learn",
            "--yes",
            "--project",
            "backend",
            "--problem",
            "PROJECTMEMORY-4318",
            "--solution",
            "Our backend workaround",
            "--json",
        ],
    );
    assert!(
        Path::new(capture["path"].as_str().unwrap()).starts_with(project.join(".lbc/knowledge"))
    );
    assert!(
        json(home.path(), &["search", "PROJECTMEMORY-4318", "--json"])
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        json(
            home.path(),
            &[
                "search",
                "PROJECTMEMORY-4318",
                "--project",
                "backend",
                "--json"
            ]
        )[0]["source_id"],
        capture["source_id"]
    );
}

#[cfg(unix)]
#[test]
fn learn_rejects_symlinked_project_stores() {
    let home = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::create_dir(home.path().join(".lbc")).unwrap();
    std::os::unix::fs::symlink(outside.path(), home.path().join(".lbc/knowledge")).unwrap();
    let output = isolated_lbc(home.path())
        .args([
            "learn",
            "--yes",
            "--problem",
            "Incident",
            "--solution",
            "Workaround",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(std::fs::read_dir(outside.path()).unwrap().next().is_none());
}
