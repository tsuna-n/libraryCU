use super::*;
use librarycube::security::git;
use std::path::Path;

fn git_command(root: &Path, args: &[&str]) -> std::process::Output {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn repository() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    git_command(root.path(), &["init", "--quiet"]);
    std::fs::create_dir(root.path().join(".lbc")).unwrap();
    root
}

#[test]
fn direct_capture_initializes_git_safety_without_setup() {
    let home = repository();
    let output = isolated_lbc(home.path())
        .args([
            "learn",
            "--yes",
            "--problem",
            "GIT-SAFETY-9148",
            "--solution",
            "A verified fix",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(home.path().join(".lbc/.gitignore").is_file());
    assert!(git::check(home.path()).unwrap().ok);
}

#[test]
fn defaults_ignore_private_records_and_future_files_but_allow_chosen_knowledge() {
    let root = repository();
    git::ensure_gitignore(root.path()).unwrap();
    for name in [
        "recovery.key",
        "fixes/fix.json",
        "proposals/fix.json",
        "history/private.json",
        "future/record.bin",
        "knowledge/private.md",
    ] {
        let path = root.path().join(".lbc").join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "private-record-content").unwrap();
    }
    assert!(git::check(root.path()).unwrap().ok);
    let files = git_command(
        root.path(),
        &["ls-files", "--others", "--exclude-standard", "--", ".lbc"],
    );
    assert_eq!(files.stdout, b".lbc/.gitignore\n");
    let ignore = root.path().join(".lbc/.gitignore");
    let mut rules = std::fs::read_to_string(&ignore).unwrap();
    rules.push_str("\n!/knowledge/\n/knowledge/*\n!/knowledge/shared.md\n");
    std::fs::write(ignore, rules).unwrap();
    std::fs::write(
        root.path().join(".lbc/knowledge/shared.md"),
        "chosen guidance",
    )
    .unwrap();
    git_command(
        root.path(),
        &["add", ".lbc/.gitignore", ".lbc/knowledge/shared.md"],
    );
    assert!(git::check(root.path()).unwrap().ok);
}

#[test]
fn ignored_but_already_tracked_private_records_are_unsafe() {
    let root = repository();
    std::fs::write(
        root.path().join(".lbc/recovery.key"),
        "never-echo-this-secret",
    )
    .unwrap();
    git_command(root.path(), &["add", ".lbc/recovery.key"]);
    git::ensure_gitignore(root.path()).unwrap();
    let report = git::check(root.path()).unwrap();
    assert!(!report.ok);
    assert!(report.detail.contains("recovery.key"));
    assert!(report.detail.contains("already tracked"));
    assert!(!report.detail.contains("never-echo-this-secret"));
}

#[test]
fn existing_rules_are_preserved_and_effective_root_rules_are_respected() {
    let root = repository();
    let ignore = root.path().join(".lbc/.gitignore");
    std::fs::write(&ignore, "# user's own rules\n").unwrap();
    git::ensure_gitignore(root.path()).unwrap();
    assert_eq!(
        std::fs::read_to_string(&ignore).unwrap(),
        "# user's own rules\n"
    );
    assert!(!git::check(root.path()).unwrap().ok);
    std::fs::write(root.path().join(".gitignore"), "/.lbc/\n").unwrap();
    assert!(git::check(root.path()).unwrap().ok);
}

#[test]
fn doctor_reports_exposure_without_changing_git_or_private_files() {
    let root = repository();
    std::fs::write(
        root.path().join(".lbc/history.json"),
        "never-echo-history-content",
    )
    .unwrap();
    let before = git_command(
        root.path(),
        &["status", "--porcelain=v1", "--untracked-files=all"],
    )
    .stdout;
    let output = isolated_lbc(root.path())
        .args(["doctor", "--json"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        report["checks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|check| check["name"] == "Git safety" && check["ok"] == false)
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("never-echo-history-content"));
    assert!(!root.path().join(".lbc/.gitignore").exists());
    assert_eq!(
        git_command(
            root.path(),
            &["status", "--porcelain=v1", "--untracked-files=all"]
        )
        .stdout,
        before
    );
    assert_eq!(
        std::fs::read_to_string(root.path().join(".lbc/history.json")).unwrap(),
        "never-echo-history-content"
    );
}

#[test]
fn non_git_projects_are_healthy_and_setup_creates_ignore_rules() {
    let root = tempfile::tempdir().unwrap();
    assert!(git::check(root.path()).unwrap().ok);
    let output = isolated_lbc(root.path())
        .env("PATH", root.path().join("no-git-installed"))
        .args(["doctor", "--json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = isolated_lbc(root.path())
        .args(["setup", "--yes", "--json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(root.path().join(".lbc/.gitignore").is_file());
}

#[test]
fn removed_but_still_tracked_private_records_are_detected_without_lbc_directory() {
    let root = repository();
    let path = root.path().join(".lbc/recovery.key");
    std::fs::write(&path, "private").unwrap();
    git_command(root.path(), &["add", ".lbc/recovery.key"]);
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(root.path().join(".lbc")).unwrap();
    assert!(!git::check(root.path()).unwrap().ok);
}

#[cfg(unix)]
#[test]
fn symlinked_knowledge_and_control_characters_are_not_safe_to_share() {
    use std::os::unix::fs::symlink;
    let root = repository();
    git::ensure_gitignore(root.path()).unwrap();
    std::fs::create_dir(root.path().join(".lbc/knowledge")).unwrap();
    let ignore = root.path().join(".lbc/.gitignore");
    let mut rules = std::fs::read_to_string(&ignore).unwrap();
    rules.push_str("!/knowledge/\n/knowledge/*\n!/knowledge/shared.md\n!/unsafe*\n");
    std::fs::write(ignore, rules).unwrap();
    symlink(
        "../../private.key",
        root.path().join(".lbc/knowledge/shared.md"),
    )
    .unwrap();
    std::fs::write(root.path().join(".lbc/unsafe\n\x1b[31m"), "private").unwrap();
    let report = git::check(root.path()).unwrap();
    assert!(!report.ok);
    assert!(report.detail.contains("shared.md"));
    assert!(!report.detail.contains('\n'));
    assert!(!report.detail.contains('\x1b'));
    git_command(root.path(), &["add", ".lbc/knowledge/shared.md"]);
    assert!(
        git::check(root.path())
            .unwrap()
            .detail
            .contains("already tracked")
    );
}

#[cfg(unix)]
#[test]
fn ignore_creation_refuses_symlinks_special_files_and_unsafe_permissions() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let root = repository();
    let outside = tempfile::tempdir().unwrap();
    let ignore = root.path().join(".lbc/.gitignore");
    symlink(outside.path().join("untouched"), &ignore).unwrap();
    assert!(git::ensure_gitignore(root.path()).is_err());
    assert!(!outside.path().join("untouched").exists());
    std::fs::remove_file(&ignore).unwrap();
    std::fs::create_dir(&ignore).unwrap();
    assert!(git::ensure_gitignore(root.path()).is_err());
    std::fs::remove_dir(&ignore).unwrap();
    std::fs::set_permissions(
        root.path().join(".lbc"),
        std::fs::Permissions::from_mode(0o777),
    )
    .unwrap();
    assert!(git::ensure_gitignore(root.path()).is_err());
    assert!(!ignore.exists());
}
