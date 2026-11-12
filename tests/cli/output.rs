use super::*;

#[test]
fn limits_keep_json_arrays_and_existing_defaults() {
    let home = tempfile::tempdir().unwrap();
    let notes = home.path().join(".lbc/knowledge");
    std::fs::create_dir_all(&notes).unwrap();
    for index in 0..12 {
        std::fs::write(
            notes.join(format!("note-{index}.md")),
            format!("---\nid: limit-{index}\ntitle: Nebulalimit guidance {index}\n---\nNebulalimit solution {index}.\n"),
        )
        .unwrap();
    }
    for command in ["search", "list"] {
        let mut args = vec![command];
        if command == "search" {
            args.push("nebulalimit");
        }
        let full = isolated_lbc(home.path())
            .args(&args)
            .arg("--json")
            .output()
            .unwrap();
        assert!(full.status.success());
        let full: serde_json::Value = serde_json::from_slice(&full.stdout).unwrap();
        assert!(full.as_array().unwrap().len() >= 12);
        let limited = isolated_lbc(home.path())
            .args(&args)
            .args(["--limit", "2", "--json", "--quiet", "--no-color"])
            .output()
            .unwrap();
        assert!(
            limited.status.success(),
            "{}",
            String::from_utf8_lossy(&limited.stderr)
        );
        assert!(limited.stderr.is_empty());
        let limited: serde_json::Value = serde_json::from_slice(&limited.stdout).unwrap();
        assert_eq!(limited.as_array().unwrap(), &full.as_array().unwrap()[..2]);
        let text = isolated_lbc(home.path())
            .args(&args)
            .args(["--limit", "12"])
            .output()
            .unwrap();
        assert!(text.status.success());
        let text = String::from_utf8_lossy(&text.stdout);
        if command == "search" {
            assert_eq!(text.matches("Nebulalimit guidance").count(), 12, "{text}");
        } else {
            assert_eq!(
                text.lines()
                    .filter(|line| line.starts_with("builtin:") || line.starts_with("project:"))
                    .count(),
                12,
                "{text}"
            );
        }
        for value in ["0", "-1", "invalid"] {
            let failed = isolated_lbc(home.path())
                .args(&args)
                .args(["--limit", value])
                .output()
                .unwrap();
            assert_eq!(failed.status.code(), Some(2));
            assert!(failed.stdout.is_empty());
        }
    }
    let text = isolated_lbc(home.path())
        .args(["search", "nebulalimit"])
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&text.stdout)
            .matches("Nebulalimit guidance")
            .count(),
        10
    );
}

#[test]
fn quiet_suppresses_warnings_but_keeps_json_and_failures() {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join(".lbc/knowledge")).unwrap();
    std::fs::write(home.path().join(".lbc/knowledge/broken.md"), "invalid").unwrap();
    let normal = isolated_lbc(home.path())
        .args(["search", "E0308", "--json"])
        .output()
        .unwrap();
    assert!(normal.status.success());
    assert!(String::from_utf8_lossy(&normal.stderr).contains("lbc index"));
    let quiet = isolated_lbc(home.path())
        .args(["--quiet", "search", "E0308", "--json"])
        .output()
        .unwrap();
    assert!(quiet.status.success());
    assert_eq!(quiet.stdout, normal.stdout);
    assert!(quiet.stderr.is_empty());
    let failed = isolated_lbc(home.path())
        .args(["search", "", "--quiet"])
        .output()
        .unwrap();
    assert!(!failed.status.success());
    assert!(failed.stdout.is_empty());
    assert!(!failed.stderr.is_empty());
}

#[test]
fn plain_output_flags_work_before_and_after_subcommands() {
    for args in [
        vec!["--no-color", "--help"],
        vec!["search", "--no-color", "--help"],
        vec!["--quiet", "config", "show", "--json"],
    ] {
        let home = tempfile::tempdir().unwrap();
        let output = isolated_lbc(home.path())
            .args(args)
            .env("CLICOLOR_FORCE", "1")
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(!output.stdout.contains(&0x1b));
        assert!(!output.stderr.contains(&0x1b));
    }
}

#[test]
fn invalid_knowledge_diagnostics_are_safe_in_text_and_json() {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join(".lbc/knowledge")).unwrap();
    std::fs::write(
        home.path().join(".lbc/knowledge/private.md"),
        "---\nid: private\ntags: unlabelled-private-marker\n---\nbody",
    )
    .unwrap();
    for args in [
        vec!["index"],
        vec!["index", "--json"],
        vec!["ask", "E0308", "--json"],
        vec!["search", "E0308"],
    ] {
        let output = isolated_lbc(home.path()).args(args).output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stdout.contains("unlabelled-private-marker"), "{stdout}");
        assert!(!stderr.contains("unlabelled-private-marker"), "{stderr}");
        if stdout.starts_with("Knowledge index") {
            assert!(!stdout.contains("invalid YAML"), "{stdout}");
            assert!(stderr.contains("invalid YAML"), "{stderr}");
        }
    }
}

#[test]
fn invalid_config_changes_do_not_print_private_values_or_overwrite_config() {
    let home = tempfile::tempdir().unwrap();
    let config = home.path().join("config.toml");
    let original = "[output]\nlanguage = 'en'\n";
    std::fs::write(&config, original).unwrap();
    for key in [
        "ai.provider",
        "scanner.max_file_size_kb",
        "unknown-private-setting",
    ] {
        let output = isolated_lbc(home.path())
            .args(["config", "set", key, "unlabelled-private-marker"])
            .env("LBC_CONFIG", &config)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(
            output.stdout.is_empty(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert!(!String::from_utf8_lossy(&output.stderr).contains("unlabelled-private-marker"));
        assert_eq!(std::fs::read_to_string(&config).unwrap(), original);
    }
}

#[test]
fn quiet_ai_fallback_preserves_machine_readable_results() {
    let home = tempfile::tempdir().unwrap();
    std::fs::write(
        home.path().join("error.log"),
        "error[E0308]: mismatched types\n",
    )
    .unwrap();
    for args in [
        vec!["ask", "E0308", "--ai", "--json"],
        vec!["explain", "error.log", "--ai", "--json"],
    ] {
        let normal = isolated_lbc(home.path()).args(&args).output().unwrap();
        assert!(normal.status.success());
        let quiet = isolated_lbc(home.path())
            .args(&args)
            .args(["--quiet", "--no-color"])
            .output()
            .unwrap();
        assert!(quiet.status.success());
        assert!(
            quiet.stderr.is_empty(),
            "{}",
            String::from_utf8_lossy(&quiet.stderr)
        );
        assert_eq!(quiet.stdout, normal.stdout);
        let report: serde_json::Value = serde_json::from_slice(&quiet.stdout).unwrap();
        assert!(
            report["ai_error"]
                .as_str()
                .unwrap()
                .contains("lbc config show")
        );
        assert!(!quiet.stdout.contains(&0x1b));
    }
}
