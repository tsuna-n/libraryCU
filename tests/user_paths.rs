use std::{fs, path::Path, process::Command};

fn cli(project: &Path, profile: &Path, args: &[&str]) -> serde_json::Value {
    let output = Command::new(
        std::env::var_os("LBC_TEST_BINARY").unwrap_or_else(|| env!("CARGO_BIN_EXE_lbc").into()),
    )
    .current_dir(project)
    .env_remove("HOME")
    .env_remove("XDG_DATA_HOME")
    .env_remove("XDG_CONFIG_HOME")
    .env_remove("LBC_CONFIG")
    .env("USERPROFILE", profile)
    .args(args)
    .output()
    .unwrap();
    assert!(output.status.success(), "{output:?}");
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn user_profile_defaults_keep_personal_knowledge_between_projects() {
    let root = tempfile::tempdir().unwrap();
    let profile = root.path().join("profile");
    let first = root.path().join("first");
    let second = root.path().join("second");
    for directory in [&profile, &first, &second] {
        fs::create_dir(directory).unwrap();
    }
    cli(&first, &profile, &["setup", "--yes", "--json"]);
    assert!(profile.join(".config/lbc/config.toml").is_file());
    assert!(profile.join(".local/share/lbc/notes").is_dir());
    assert!(!first.join(".config").exists());
    cli(
        &first,
        &profile,
        &[
            "learn",
            "--yes",
            "--user",
            "--problem",
            "PROFILECASE4318",
            "--solution",
            "Personal knowledge follows the user",
            "--json",
        ],
    );
    let results = cli(&second, &profile, &["search", "PROFILECASE4318", "--json"]);
    assert!(
        results[0]["source_id"]
            .as_str()
            .unwrap()
            .starts_with("user:")
    );
    let history = cli(&second, &profile, &["history", "show", "--json"]);
    assert_eq!(
        Path::new(history["path"].as_str().unwrap()),
        profile.join(".local/share/lbc/history/default.json")
    );
}

#[test]
fn existing_relative_user_stores_and_config_are_preserved() {
    let root = tempfile::tempdir().unwrap();
    let profile = root.path().join("profile");
    let project = root.path().join("project");
    fs::create_dir(&profile).unwrap();
    let config = project.join(".config/lbc/config.toml");
    fs::create_dir_all(config.parent().unwrap()).unwrap();
    fs::write(&config, "[output]\nlanguage = 'th'\n").unwrap();
    let note = project.join(".local/share/lbc/notes/legacy.md");
    fs::create_dir_all(note.parent().unwrap()).unwrap();
    let original =
        "---\nid: legacy\ntitle: LEGACYPROFILE4318\nkind: note\n---\nKeep original knowledge.\n";
    fs::write(&note, original).unwrap();
    cli(&project, &profile, &["setup", "--yes", "--json"]);
    assert_eq!(fs::read_to_string(note).unwrap(), original);
    let stored: toml::Value = fs::read_to_string(config).unwrap().parse().unwrap();
    assert_eq!(stored["output"]["language"].as_str(), Some("th"));
    assert!(!profile.join(".config").exists());
    assert!(!profile.join(".local").exists());
    let results = cli(
        &project,
        &profile,
        &["search", "LEGACYPROFILE4318", "--json"],
    );
    assert_eq!(results[0]["source_id"], "user:legacy");
}
