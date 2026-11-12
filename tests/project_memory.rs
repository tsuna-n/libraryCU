use std::{fs, path::Path, process::Command};

use librarycube::{
    answer,
    config::settings::{OutputConfig, ScannerConfig},
    diagnostics::{self, Confidence},
    knowledge::retrieval::{RetrievalScope, retrieve_with_scope},
};

fn note(root: &Path, id: &str, metadata: &str, body: &str) {
    let directory = root.join(".lbc/knowledge");
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join(format!("{id}.md")),
        format!("---\nid: {id}\n{metadata}---\n{body}\n"),
    )
    .unwrap();
}

#[test]
fn project_memory_is_isolated_even_when_projects_use_the_same_id() {
    let home = tempfile::tempdir().unwrap();
    let first = home.path().join("first");
    let second = home.path().join("second");
    note(
        &first,
        "deployment",
        "",
        "# Deployment\nUse the first service.",
    );
    note(
        &second,
        "deployment",
        "",
        "# Deployment\nUse the second service.",
    );
    note(home.path(), "parent", "", "# Deployment\nParent guidance.");
    for (root, expected) in [(&first, "first service"), (&second, "second service")] {
        let report =
            retrieve_with_scope(root, "deployment", "en", RetrievalScope::Project).unwrap();
        assert_eq!(report.results.len(), 1);
        assert_eq!(report.results[0].source_id, "project:deployment");
        assert!(report.results[0].excerpt.contains(expected));
        assert!(report.invalid.is_empty());
    }
    let nested = first.join("src/deep");
    fs::create_dir_all(&nested).unwrap();
    assert!(
        retrieve_with_scope(&nested, "deployment", "en", RetrievalScope::Project)
            .unwrap()
            .results
            .is_empty()
    );
}

#[test]
fn project_only_uses_legacy_notes_and_preserves_external_overrides() {
    let home = tempfile::tempdir().unwrap();
    note(
        home.path(),
        "local-fix",
        "overrides: builtin:rust-e0382\nerror_code: E0382\n",
        "# Local ownership fix\nKeep the original value.",
    );
    fs::create_dir(home.path().join("knowledge")).unwrap();
    fs::write(
        home.path().join("knowledge/legacy.md"),
        "---\nid: legacy\n---\n# Legacy setup\nLocal legacy setup.",
    )
    .unwrap();
    let report = retrieve_with_scope(home.path(), "E0382", "en", RetrievalScope::Project).unwrap();
    assert_eq!(report.results.len(), 1);
    assert_eq!(report.results[0].source_id, "project:local-fix");
    assert!(report.invalid.is_empty());
    assert_eq!(
        retrieve_with_scope(home.path(), "legacy setup", "en", RetrievalScope::Project)
            .unwrap()
            .results[0]
            .source_id,
        "project:legacy"
    );
    assert!(
        retrieve_with_scope(home.path(), "E0308", "en", RetrievalScope::Project)
            .unwrap()
            .results
            .is_empty()
    );
}

#[test]
fn project_only_excludes_shared_stores_and_their_invalid_paths_in_every_reader() {
    let home = tempfile::tempdir().unwrap();
    let project = home.path().join("project");
    note(
        &project,
        "local",
        "",
        "# Startup failure\nLocal startup failure.",
    );
    let data = home.path().join("data/lbc");
    fs::create_dir_all(data.join("notes")).unwrap();
    fs::write(
        data.join("notes/shared.md"),
        "---\nid: shared\n---\n# Startup failure\nShared startup failure.",
    )
    .unwrap();
    fs::write(
        data.join("notes/private-invalid.md"),
        "---\nid: [invalid\n---\n",
    )
    .unwrap();
    let package = data.join("knowledge/shared");
    fs::create_dir_all(&package).unwrap();
    fs::write(
        package.join("package.toml"),
        "name = 'shared'\nversion = '1.0.0'\n",
    )
    .unwrap();
    fs::write(
        package.join("shared.md"),
        "---\nid: shared\n---\n# Startup failure\nPackage startup failure.",
    )
    .unwrap();
    let input = home.path().join("error.log");
    fs::write(&input, "startup failure").unwrap();
    for reader in ["search", "ask", "explain"] {
        for project_only in [false, true] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_lbc"));
            command
                .current_dir(&project)
                .env("XDG_DATA_HOME", home.path().join("data"))
                .env("XDG_CONFIG_HOME", home.path().join("config"))
                .env("XDG_CACHE_HOME", home.path().join("cache"))
                .env_remove("LBC_CONFIG")
                .arg(reader);
            if reader == "explain" {
                command.arg(&input);
            } else {
                command.arg("startup failure");
            }
            command.arg("--project").arg(&project).arg("--json");
            if project_only {
                command.arg("--project-only");
            }
            let output = command.output().unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            let results = match reader {
                "ask" => &value["passages"],
                "explain" => &value["knowledge"],
                _ => &value,
            }
            .as_array()
            .unwrap();
            assert!(
                results
                    .iter()
                    .any(|item| item["source_id"] == "project:local")
            );
            if project_only {
                assert_eq!(results.len(), 1, "{reader}: {value}");
                assert!(!String::from_utf8_lossy(&output.stdout).contains("private-invalid"));
                assert!(!String::from_utf8_lossy(&output.stderr).contains("private-invalid"));
            } else {
                assert!(
                    results
                        .iter()
                        .any(|item| item["source_id"] == "user:shared")
                );
                assert!(
                    results
                        .iter()
                        .any(|item| item["source_id"] == "package:shared:shared")
                );
                assert!(
                    String::from_utf8_lossy(&output.stdout).contains("private-invalid")
                        || String::from_utf8_lossy(&output.stderr).contains("private-invalid")
                );
            }
        }
    }
}

#[test]
fn project_context_ranks_compatible_language_tool_and_framework() {
    let home = tempfile::tempdir().unwrap();
    fs::write(
        home.path().join("Cargo.toml"),
        "[package]\nname = 'memory'\nversion = '0.1.0'\n[dependencies]\naxum = '0.8'\n",
    )
    .unwrap();
    note(
        home.path(),
        "a-python",
        "language: python\ntool: pytest\nframework: Django\n",
        "# Startup failure\nCheck the startup configuration.",
    );
    note(
        home.path(),
        "z-rust",
        "language: rust\ntool: cargo\nframework: Axum\n",
        "# Startup failure\nCheck the startup configuration.",
    );
    let report = retrieve_with_scope(
        home.path(),
        "startup failure",
        "en",
        RetrievalScope::Project,
    )
    .unwrap();
    assert_eq!(report.results[0].source_id, "project:z-rust");
    for reason in [
        "project language: Rust",
        "project tool: Cargo",
        "project framework: Axum",
    ] {
        assert!(
            report.results[0]
                .ranking_reasons
                .iter()
                .any(|value| value == reason),
            "{reason}"
        );
    }
}

#[test]
fn project_context_cannot_promote_weak_overlap_over_codes_or_identifiers() {
    let home = tempfile::tempdir().unwrap();
    fs::write(
        home.path().join("Cargo.toml"),
        "[package]\nname = 'memory'\nversion = '0.1.0'\n",
    )
    .unwrap();
    note(
        home.path(),
        "weak",
        "language: rust\ntool: cargo\n",
        "# Startup\nCheck startup configuration.",
    );
    note(
        home.path(),
        "exact",
        "error_code: ERR_WIDGET\n",
        "# Widget error\nA precise widget failure.",
    );
    for query in [
        "ERR_WIDGET startup configuration",
        "project:exact startup configuration",
    ] {
        let report =
            retrieve_with_scope(home.path(), query, "en", RetrievalScope::Project).unwrap();
        assert_eq!(report.results[0].source_id, "project:exact");
    }
    assert!(
        retrieve_with_scope(
            home.path(),
            "startup configuration vanished unexpectedly",
            "en",
            RetrievalScope::Project,
        )
        .unwrap()
        .results
        .is_empty()
    );
}

#[test]
fn historical_success_is_not_confirmation_for_the_current_problem() {
    let home = tempfile::tempdir().unwrap();
    note(
        home.path(),
        "startup",
        "kind: troubleshooting\nverification_status: recorded-check\n",
        "# Startup failure\n## Problem\nStartup failure.\n## Solution\nRestart the service.\n## Verification\nThe previous deployment passed its check.",
    );
    let report = answer::answer_with_scope(
        "startup failure",
        home.path(),
        &OutputConfig::default(),
        &ScannerConfig::default(),
        RetrievalScope::Project,
    )
    .unwrap();
    assert_eq!(report.answer_status, "retrieved_guidance");
    assert!(report.offline_answer.contains("not rerun"));
    assert!(
        report.passages[0]
            .verification_status
            .contains("did not rerun")
    );
    let request = answer::build_ai_request(&report, "test", &[]);
    assert!(
        request
            .system
            .contains("Historical knowledge is not verification of the current problem")
    );
    let explained = diagnostics::explain_with_scope(
        "startup failure",
        home.path(),
        &ScannerConfig::default(),
        RetrievalScope::Project,
    )
    .unwrap();
    assert_eq!(explained.confidence, Confidence::RetrievedKnowledge);
    assert!(explained.cause.contains("has not been verified"));
    let request = librarycube::ai::build_request(&explained, "startup failure", "test");
    assert!(
        request
            .system
            .contains("Historical knowledge is not verification of the current problem")
    );
}

#[cfg(unix)]
#[test]
fn project_scope_rejects_symlink_roots_and_skips_symlink_knowledge() {
    let home = tempfile::tempdir().unwrap();
    let first = home.path().join("first");
    let second = home.path().join("second");
    fs::create_dir(&first).unwrap();
    note(&second, "secret", "", "# Private memory\nSibling content.");
    std::os::unix::fs::symlink(&second, home.path().join("link")).unwrap();
    assert!(
        retrieve_with_scope(
            &home.path().join("link"),
            "private memory",
            "en",
            RetrievalScope::Project
        )
        .is_err()
    );
    fs::create_dir(first.join(".lbc")).unwrap();
    std::os::unix::fs::symlink(second.join(".lbc/knowledge"), first.join(".lbc/knowledge"))
        .unwrap();
    let report =
        retrieve_with_scope(&first, "private memory", "en", RetrievalScope::Project).unwrap();
    assert!(report.results.is_empty());
    assert_eq!(report.invalid.len(), 1);
}
