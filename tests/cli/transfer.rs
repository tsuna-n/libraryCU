use super::*;
use std::{fs, path::Path};

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

fn note(id: &str, body: &str) -> String {
    format!(
        "---\nid: {id}\nlegacy_extra: keep-exactly\ncreated_at_unix: 123\n---\n# Local note\n\n{body}\n"
    )
}

#[test]
fn backup_export_restore_import_preserve_legacy_metadata_ids_and_scope() {
    let source = tempfile::tempdir().unwrap();
    let legacy = note("legacy-id", "Historic fix ภาษาไทย").replace('\n', "\r\n");
    let modern = note("modern-id", "Daily fix");
    for (path, content) in [
        ("knowledge/nested/old-name.md", legacy.as_str()),
        (".lbc/knowledge/.archive/another-name.md", modern.as_str()),
        (".lbc/recovery.key", "private-key-fixture"),
        (".lbc/history/private.json", "private-history-fixture"),
        (".lbc/proposals/private.json", "private-record-fixture"),
    ] {
        let path = source.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
    success(
        source.path(),
        &["backup", "create", "backup.json", "--json"],
    );
    success(source.path(), &["export", "export.json", "--json"]);
    let bytes = fs::read(source.path().join("backup.json")).unwrap();
    assert_eq!(bytes, fs::read(source.path().join("export.json")).unwrap());
    let rendered = String::from_utf8(bytes.clone()).unwrap();
    for private in [
        "private-key-fixture",
        "private-history-fixture",
        "private-record-fixture",
        source.path().to_str().unwrap(),
    ] {
        assert!(!rendered.contains(private));
    }
    let dest = tempfile::tempdir().unwrap();
    let archive = source.path().join("backup.json");
    let restored = success(
        dest.path(),
        &[
            "backup",
            "restore",
            archive.to_str().unwrap(),
            "--yes",
            "--json",
        ],
    );
    assert_eq!(restored["added"].as_array().unwrap().len(), 2);
    let store = dest.path().join(".lbc/knowledge");
    assert_eq!(
        fs::read_to_string(store.join("legacy-id.md")).unwrap(),
        legacy
    );
    assert_eq!(
        fs::read_to_string(store.join("modern-id.md")).unwrap(),
        modern
    );
    let retry = success(
        dest.path(),
        &["import", archive.to_str().unwrap(), "--yes", "--json"],
    );
    assert_eq!(retry["duplicates"].as_array().unwrap().len(), 2);
    assert!(retry["added"].as_array().unwrap().is_empty());
    assert!(dest.path().join(".lbc/.gitignore").is_file());
    success(
        source.path(),
        &["backup", "create", ".lbc/backups/snapshot.json", "--json"],
    );
    let local_archive = source.path().join(".lbc/backups/snapshot.json");
    let local_retry = success(
        dest.path(),
        &[
            "backup",
            "restore",
            local_archive.to_str().unwrap(),
            "--yes",
            "--json",
        ],
    );
    assert_eq!(local_retry["duplicates"].as_array().unwrap().len(), 2);
    assert!(source.path().join(".lbc/.gitignore").is_file());
    let before = fs::read(source.path().join("backup.json")).unwrap();
    assert!(
        !run(source.path(), &["backup", "create", "backup.json"])
            .status
            .success()
    );
    assert_eq!(fs::read(source.path().join("backup.json")).unwrap(), before);
}

#[test]
fn restore_conflicts_and_cancellation_preserve_every_existing_note() {
    let source = tempfile::tempdir().unwrap();
    fs::create_dir(source.path().join("knowledge")).unwrap();
    fs::write(
        source.path().join("knowledge/one.md"),
        note("one", "Replacement fix"),
    )
    .unwrap();
    fs::write(
        source.path().join("knowledge/two.md"),
        note("two", "Second fix"),
    )
    .unwrap();
    success(source.path(), &["export", "bundle.json", "--json"]);
    let archive = source.path().join("bundle.json");
    let dest = tempfile::tempdir().unwrap();
    let old = dest.path().join("knowledge/legacy-name.md");
    fs::create_dir_all(old.parent().unwrap()).unwrap();
    let original = note("one", "Original fix");
    fs::write(&old, &original).unwrap();
    let blocked = run(
        dest.path(),
        &["backup", "restore", archive.to_str().unwrap(), "--yes"],
    );
    assert!(!blocked.status.success());
    assert!(String::from_utf8_lossy(&blocked.stderr).contains("--replace"));
    assert!(!dest.path().join(".lbc").exists());
    for input in ["\n", ":cancel\n", "no\n", ""] {
        let output = interactive(
            dest.path(),
            &["import", archive.to_str().unwrap(), "--replace"],
            input,
        );
        assert_eq!(output.status.success(), !input.is_empty());
        assert_eq!(fs::read_to_string(&old).unwrap(), original);
        assert!(!dest.path().join(".lbc").exists());
    }
    let output = interactive(
        dest.path(),
        &["import", archive.to_str().unwrap(), "--replace"],
        "y\n",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        fs::read_to_string(&old)
            .unwrap()
            .contains("Replacement fix")
    );
    assert!(!dest.path().join(".lbc/knowledge/one.md").exists());
    assert!(dest.path().join(".lbc/knowledge/two.md").is_file());
}

#[test]
fn corrupted_bundles_are_rejected_before_any_store_is_created() {
    let source = tempfile::tempdir().unwrap();
    fs::create_dir(source.path().join("knowledge")).unwrap();
    fs::write(
        source.path().join("knowledge/valid.md"),
        note("valid", "Original fix"),
    )
    .unwrap();
    success(source.path(), &["export", "bundle.json", "--json"]);
    let valid: serde_json::Value =
        serde_json::from_slice(&fs::read(source.path().join("bundle.json")).unwrap()).unwrap();
    let mut tampered = valid.clone();
    tampered["entries"][0]["markdown"] = serde_json::json!(note("valid", "Tampered fix"));
    let mut version = valid.clone();
    version["version"] = serde_json::json!(999);
    let mut duplicate = valid.clone();
    duplicate["entries"]
        .as_array_mut()
        .unwrap()
        .push(valid["entries"][0].clone());
    for content in [
        "{truncated".to_owned(),
        tampered.to_string(),
        version.to_string(),
        duplicate.to_string(),
    ] {
        let dest = tempfile::tempdir().unwrap();
        fs::write(dest.path().join("corrupt.json"), content).unwrap();
        let output = run(dest.path(), &["backup", "restore", "corrupt.json", "--yes"]);
        assert!(!output.status.success());
        assert!(!dest.path().join(".lbc").exists());
    }
}

#[test]
fn personal_knowledge_moves_between_machine_profiles_without_project_writes() {
    let source = tempfile::tempdir().unwrap();
    success(
        source.path(),
        &[
            "learn",
            "--user",
            "--yes",
            "--id",
            "personal-id",
            "--problem",
            "PORTABLE-CASE-9058",
            "--solution",
            "A proven personal fix",
            "--json",
        ],
    );
    success(
        source.path(),
        &["export", "personal.json", "--user", "--json"],
    );
    let archive = source.path().join("personal.json");
    let dest = tempfile::tempdir().unwrap();
    let report = success(
        dest.path(),
        &[
            "import",
            archive.to_str().unwrap(),
            "--user",
            "--yes",
            "--json",
        ],
    );
    assert_eq!(report["added"][0], "personal-id");
    assert!(!dest.path().join(".lbc").exists());
    assert!(dest.path().join("data/lbc/notes/personal-id.md").is_file());
    assert_eq!(
        fs::read(source.path().join("data/lbc/notes/personal-id.md")).unwrap(),
        fs::read(dest.path().join("data/lbc/notes/personal-id.md")).unwrap()
    );
}

#[test]
fn legacy_markdown_directories_and_similar_entries_are_reviewable() {
    let home = tempfile::tempdir().unwrap();
    let directory = home.path().join("old-notes");
    fs::create_dir(&directory).unwrap();
    fs::write(
        directory.join("first.md"),
        note("first", "Shared recorded fix"),
    )
    .unwrap();
    fs::write(
        directory.join("second.md"),
        note("second", "Shared recorded fix"),
    )
    .unwrap();
    fs::create_dir(directory.join(".archive")).unwrap();
    fs::write(
        directory.join(".archive/third.md"),
        note("third", "Hidden legacy note"),
    )
    .unwrap();
    fs::create_dir(directory.join("runtime")).unwrap();
    fs::write(
        directory.join("runtime/fourth.md"),
        note("fourth", "Runtime debugging note"),
    )
    .unwrap();
    let output = run(home.path(), &["import", "old-notes", "--yes", "--json"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["added"].as_array().unwrap().len(), 4);
    assert!(!report["similar"].as_array().unwrap().is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Similar knowledge"));
    assert_eq!(
        fs::read_to_string(home.path().join(".lbc/knowledge/first.md")).unwrap(),
        note("first", "Shared recorded fix")
    );
}
