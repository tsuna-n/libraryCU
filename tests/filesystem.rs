use librarycube::{
    knowledge::{AddEntry, add_entry},
    security::files::read_text,
};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("lbc-file-safety-{}-{nonce}", std::process::id()));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(unix)]
#[test]
fn project_add_rejects_parent_symlink_before_creating_anything() {
    let fixture = Fixture::new();
    let project = fixture.0.join("project");
    let outside = fixture.0.join("outside");
    fs::create_dir(&project).unwrap();
    fs::create_dir(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, project.join(".lbc")).unwrap();
    let result = add_entry(AddEntry {
        id: Some("safe-note"),
        title: "Safe note",
        kind: "note",
        body: "fixture body",
        project: Some(&project),
        overrides: None,
    });
    assert!(result.is_err());
    assert_eq!(
        fs::read_dir(&outside).unwrap().count(),
        0,
        "rejected writes must not create an outside directory"
    );
}

#[cfg(unix)]
#[test]
fn symlinked_project_store_cannot_supply_knowledge() {
    let fixture = Fixture::new();
    let project = fixture.0.join("project");
    let outside = fixture.0.join("outside/knowledge");
    fs::create_dir(&project).unwrap();
    fs::create_dir_all(&outside).unwrap();
    fs::write(
        outside.join("private.md"),
        "---\nid: outside-private\ntitle: Outside private\n---\nOUTSIDE-PRIVATE-MATERIAL",
    )
    .unwrap();
    std::os::unix::fs::symlink(outside.parent().unwrap(), project.join(".lbc")).unwrap();
    let documents = librarycube::knowledge::loader::load_documents_with_data_dir(
        &project,
        &fixture.0.join("packages"),
    )
    .unwrap();
    assert!(
        documents
            .iter()
            .all(|doc| doc.metadata.id != "outside-private")
    );
}

#[test]
fn reader_rejects_non_regular_and_oversized_inputs() {
    let fixture = Fixture::new();
    assert!(read_text(&fixture.0, 1024).is_err());
    let path = fixture.0.join("large.txt");
    fs::write(&path, "x".repeat(1025)).unwrap();
    assert!(read_text(&path, 1024).is_err());
    fs::write(&path, "normal text").unwrap();
    assert_eq!(read_text(&path, 1024).unwrap(), "normal text");
}

#[test]
fn add_rejects_an_id_in_a_differently_named_document() {
    let fixture = Fixture::new();
    let store = fixture.0.join(".lbc/knowledge");
    fs::create_dir_all(&store).unwrap();
    fs::write(
        store.join("renamed.md"),
        "---\nid: existing-id\ntitle: Existing note\n---\nORIGINAL-CONTENT",
    )
    .unwrap();
    let result = add_entry(AddEntry {
        id: Some("existing-id"),
        title: "Duplicate",
        kind: "note",
        body: "new body",
        project: Some(&fixture.0),
        overrides: None,
    });
    assert!(result.is_err());
    assert!(!store.join("existing-id.md").exists());
}

#[test]
fn add_never_saves_a_document_too_large_for_the_loader() {
    let fixture = Fixture::new();
    let huge_title = "x".repeat(256 * 1024);
    let result = add_entry(AddEntry {
        id: Some("oversized"),
        title: &huge_title,
        kind: "note",
        body: "body",
        project: Some(&fixture.0),
        overrides: None,
    });
    assert!(result.is_err());
    assert!(!fixture.0.join(".lbc/knowledge/oversized.md").exists());
}

#[cfg(unix)]
#[test]
fn fifo_input_fails_without_waiting_for_a_writer() {
    use std::os::unix::ffi::OsStrExt;
    let fixture = Fixture::new();
    let path = fixture.0.join("pipe");
    let cpath = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
    // SAFETY: cpath is NUL-terminated and remains alive through mkfifo.
    assert_eq!(unsafe { libc::mkfifo(cpath.as_ptr(), 0o600) }, 0);
    assert!(
        read_text(&path, 1024)
            .unwrap_err()
            .to_string()
            .contains("regular file")
    );
}
