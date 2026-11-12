use super::isolated_lbc;

#[test]
fn invalid_config_errors_hide_source_values_and_offer_recovery() {
    for content in [
        "[ai]\nmodel = \"unlabelled-private-value\n",
        "[scanner]\nmax_file_size_kb = \"unlabelled-private-value\"\n",
        "[ai]\nprovider = \"unlabelled-private-value\"\n",
    ] {
        let home = tempfile::tempdir().unwrap();
        let config = home.path().join("config.toml");
        std::fs::write(&config, content).unwrap();
        let output = isolated_lbc(home.path())
            .args(["config", "show", "--json"])
            .env("LBC_CONFIG", &config)
            .env("LC_ALL", "en_US.UTF-8")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.contains("unlabelled-private-value"), "{stderr}");
        assert!(stderr.contains("Fix:"), "{stderr}");
        assert!(stderr.contains("lbc config show"), "{stderr}");
    }
}

#[test]
fn missing_files_offer_localized_actionable_errors_on_stderr() {
    for (config_language, locale, label) in [
        ("en", "th_TH.UTF-8", "Fix:"),
        ("th", "en_US.UTF-8", "วิธีแก้:"),
        ("auto", "th_TH.UTF-8", "วิธีแก้:"),
    ] {
        let home = tempfile::tempdir().unwrap();
        let config = home.path().join("config.toml");
        std::fs::write(
            &config,
            format!("[output]\nlanguage = '{config_language}'\n"),
        )
        .unwrap();
        let output = isolated_lbc(home.path())
            .args(["explain", "missing-error.log", "--json"])
            .env("LBC_CONFIG", &config)
            .env("LC_ALL", locale)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(label), "{stderr}");
        assert!(stderr.contains("lbc explain"), "{stderr}");
    }
}

#[test]
fn parser_errors_never_echo_invalid_values_and_keep_usage_status() {
    for args in [
        vec!["setup", "--provider", "unlabelled-private-value"],
        vec![
            "doctor",
            "--connectivity-timeout",
            "unlabelled-private-value",
        ],
        vec!["--unlabelled-private-value"],
    ] {
        let home = tempfile::tempdir().unwrap();
        let output = isolated_lbc(home.path()).args(args).output().unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.contains("unlabelled-private-value"), "{stderr}");
        assert!(stderr.contains("lbc --help"), "{stderr}");
    }
}

#[test]
fn parser_errors_identify_required_fields_and_invalid_options_safely() {
    for (args, expected) in [
        (vec!["add", "--stdin"], "--title"),
        (
            vec!["list", "--limit", "unlabelled-private-value"],
            "--limit",
        ),
        (
            vec!["setup", "--provider", "unlabelled-private-value"],
            "openai",
        ),
    ] {
        let home = tempfile::tempdir().unwrap();
        let output = isolated_lbc(home.path()).args(args).output().unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(expected), "{stderr}");
        assert!(!stderr.contains("unlabelled-private-value"), "{stderr}");
    }
}

#[test]
fn no_color_applies_to_help_and_usage_errors() {
    let home = tempfile::tempdir().unwrap();
    for args in [
        vec!["--no-color", "--help"],
        vec!["--no-color", "--unknown"],
    ] {
        let output = isolated_lbc(home.path())
            .args(args)
            .env("CLICOLOR_FORCE", "1")
            .output()
            .unwrap();
        assert!(!output.stdout.contains(&0x1b));
        assert!(!output.stderr.contains(&0x1b));
    }
    let output = isolated_lbc(home.path())
        .arg("--help")
        .env("NO_COLOR", "")
        .env("CLICOLOR_FORCE", "1")
        .output()
        .unwrap();
    assert!(!output.stdout.contains(&0x1b));
    assert!(!output.stderr.contains(&0x1b));
}
