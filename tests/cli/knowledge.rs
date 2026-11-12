use super::*;
use std::fs;

fn json(home: &std::path::Path, args: &[&str]) -> serde_json::Value {
    let output = isolated_lbc(home).args(args).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn validation_and_review_warn_without_changing_legacy_or_invalid_files() {
    let home = tempfile::tempdir().unwrap();
    let store = home.path().join("knowledge");
    fs::create_dir(&store).unwrap();
    let old = "---\r\nid: stable\r\ncustom: keep\r\nupdated_at_unix: 1\r\nreferences: [user:missing]\r\n---\r\n# Legacy\r\nBody.\r\n";
    let bad = "---\nid: [invalid\n---\nbody";
    fs::write(store.join("old.md"), old).unwrap();
    fs::write(store.join("bad.md"), bad).unwrap();
    let report = json(home.path(), &["knowledge", "validate", "--json"]);
    let warnings = report["warnings"].as_array().unwrap();
    for code in ["invalid_document", "missing_reference", "stale_document"] {
        assert!(
            warnings.iter().any(|warning| warning["code"] == code),
            "{report}"
        );
    }
    let review = json(
        home.path(),
        &["knowledge", "review", "--max-age-days", "30", "--json"],
    );
    assert!(
        review["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| warning["source_id"] == "project:stable")
    );
    assert_eq!(fs::read_to_string(store.join("old.md")).unwrap(), old);
    assert_eq!(fs::read_to_string(store.join("bad.md")).unwrap(), bad);
    assert!(!home.path().join(".lbc").exists());
    assert!(!home.path().join("data").exists());
}

#[test]
fn bundled_packages_install_outside_checkout_and_require_confirmation_to_remove() {
    let home = tempfile::tempdir().unwrap();
    let available = json(home.path(), &["knowledge", "list", "--bundled", "--json"]);
    assert!(
        available
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["name"] == "python-fastapi-basics")
    );
    let installed = isolated_lbc(home.path())
        .args(["knowledge", "install", "--bundled", "python-fastapi-basics"])
        .output()
        .unwrap();
    assert!(
        installed.status.success(),
        "{}",
        String::from_utf8_lossy(&installed.stderr)
    );
    let packages = json(home.path(), &["knowledge", "list", "--json"]);
    assert_eq!(packages[0]["name"], "python-fastapi-basics");
    assert_eq!(packages[0]["integrity"], "verified");
    assert_eq!(packages[0]["documents"], 40);
    let root = home.path().join("data/lbc/knowledge/python-fastapi-basics");
    let checksum = fs::read(root.join("SHA256SUMS")).unwrap();
    let duplicate = isolated_lbc(home.path())
        .args(["knowledge", "install", "--bundled", "python-fastapi-basics"])
        .output()
        .unwrap();
    assert!(!duplicate.status.success());
    assert_eq!(fs::read(root.join("SHA256SUMS")).unwrap(), checksum);
    fs::write(root.join("authored.md"), "User authored text").unwrap();
    let refused = isolated_lbc(home.path())
        .args(["knowledge", "remove", "python-fastapi-basics"])
        .output()
        .unwrap();
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("--yes"));
    assert_eq!(
        fs::read_to_string(root.join("authored.md")).unwrap(),
        "User authored text"
    );
    let removed = isolated_lbc(home.path())
        .args(["knowledge", "remove", "python-fastapi-basics", "--yes"])
        .output()
        .unwrap();
    assert!(
        removed.status.success(),
        "{}",
        String::from_utf8_lossy(&removed.stderr)
    );
    assert!(!root.exists());
}

#[test]
fn rejects_remote_and_unknown_package_sources_without_creating_stores() {
    let home = tempfile::tempdir().unwrap();
    for args in [
        vec!["knowledge", "install", "https://example.invalid/package"],
        vec!["knowledge", "install", "--bundled", "unknown"],
    ] {
        let output = isolated_lbc(home.path()).args(args).output().unwrap();
        assert!(!output.status.success());
    }
    assert!(!home.path().join("data").exists());
}
