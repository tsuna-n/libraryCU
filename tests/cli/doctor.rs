use super::*;

#[test]
fn fresh_doctor_explains_optional_stores_without_creating_them() {
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
    assert_eq!(keys, ["checks", "health_scope", "healthy"]);
    assert!(report["healthy"].is_boolean());
    assert_eq!(report["health_scope"], "local checks only");
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
            "Git safety",
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
    std::fs::write(home.path().join("knowledge"), "invalid store type").unwrap();
    for language in ["th", "auto"] {
        std::fs::write(
            home.path().join("config/lbc/config.toml"),
            format!("[output]\nlanguage = '{language}'\n"),
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
        assert!(stdout.contains("ตรวจสอบในเครื่องเท่านั้น"), "{stdout}");
    }
}

#[test]
fn doctor_ignores_unrecognized_legacy_provider_values() {
    let home = tempfile::tempdir().unwrap();
    let config = home.path().join("config/lbc/config.toml");
    std::fs::create_dir_all(config.parent().unwrap()).unwrap();
    std::fs::write(
        config,
        "[ai]\nprovider = 'private-unlabelled-provider-value'\nmodel = { obsolete = true }\n",
    )
    .unwrap();
    let output = isolated_lbc(home.path())
        .args(["doctor", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
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
