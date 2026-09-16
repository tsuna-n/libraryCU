use librarycube::security::{permissions, storage};
use librarycube::{
    knowledge::{AddEntry, add_entry},
    security::files::read_text,
};
use std::{fs, path::Path};

struct Fixture(tempfile::TempDir);
impl Fixture {
    fn new() -> Self {
        Self(
            tempfile::Builder::new()
                .prefix("lbc-file-safety-")
                .tempdir()
                .unwrap(),
        )
    }

    fn path(&self) -> &Path {
        self.0.path()
    }
}

#[cfg(unix)]
#[test]
fn project_add_rejects_parent_symlink_before_creating_anything() {
    let fixture = Fixture::new();
    let project = fixture.path().join("project");
    let outside = fixture.path().join("outside");
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
    let project = fixture.path().join("project");
    let outside = fixture.path().join("outside/knowledge");
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
        &fixture.path().join("packages"),
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
    assert!(read_text(fixture.path(), 1024).is_err());
    let path = fixture.path().join("large.txt");
    fs::write(&path, "x".repeat(1025)).unwrap();
    assert!(read_text(&path, 1024).is_err());
    fs::write(&path, "normal text").unwrap();
    assert_eq!(read_text(&path, 1024).unwrap(), "normal text");
}

#[test]
fn add_rejects_an_id_in_a_differently_named_document() {
    let fixture = Fixture::new();
    let store = fixture.path().join(".lbc/knowledge");
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
        project: Some(fixture.path()),
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
        project: Some(fixture.path()),
        overrides: None,
    });
    assert!(result.is_err());
    assert!(!fixture.path().join(".lbc/knowledge/oversized.md").exists());
}

#[cfg(unix)]
#[test]
fn fifo_input_fails_without_waiting_for_a_writer() {
    use std::os::unix::ffi::OsStrExt;
    let fixture = Fixture::new();
    let path = fixture.path().join("pipe");
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

#[test]
fn safe_store_lock_and_private_atomic_write_roundtrip() {
    let fixture = Fixture::new();
    let path = fixture.path().join("private.json");
    let lock = storage::lock_exclusive(&fixture.path().join("store.lock")).unwrap();
    storage::atomic_replace(&path, b"private fixture", true).unwrap();
    permissions::validate_path(&path, true).unwrap();
    assert_eq!(
        librarycube::security::files::read_store_text(&path, 1024, true).unwrap(),
        "private fixture"
    );
    drop(lock);
    assert!(storage::lock_exclusive(&fixture.path().join("store.lock")).is_ok());
}

#[cfg(unix)]
#[test]
fn unsafe_shared_store_and_ancestor_are_rejected_without_creating_children() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    let shared = fixture.path().join("shared");
    fs::create_dir(&shared).unwrap();
    for mode in [0o775, 0o777, 0o1777] {
        fs::set_permissions(&shared, fs::Permissions::from_mode(mode)).unwrap();
        assert!(storage::lock_exclusive(&shared.join("store.lock")).is_err());
        assert!(storage::atomic_replace(&shared.join("private"), b"secret", true).is_err());
        assert!(storage::lock_exclusive(&shared.join("nested/store.lock")).is_err());
        assert!(!shared.join("nested").exists());
        assert_eq!(fs::read_dir(&shared).unwrap().count(), 0);
    }
    fs::set_permissions(&shared, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(storage::lock_exclusive(&shared.join("store.lock")).is_ok());
}

#[cfg(unix)]
#[test]
fn unsafe_existing_file_is_preserved_and_private_reads_reject_public_modes() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    let path = fixture.path().join("value");
    fs::write(&path, "original").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o666)).unwrap();
    assert!(storage::atomic_replace(&path, b"replacement", true).is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), "original");
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o666
    );
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(librarycube::security::files::read_store_text(&path, 1024, true).is_err());
    // Public read-only knowledge/config compatibility does not grant write trust.
    assert!(librarycube::security::files::read_store_text(&path, 1024, false).is_ok());
}

#[cfg(target_os = "linux")]
fn set_linux_acl(path: &Path, default: bool) {
    use std::os::{fd::AsRawFd, unix::fs::PermissionsExt};
    let file = fs::File::open(path).unwrap();
    let mut bytes = 2u32.to_le_bytes().to_vec();
    // Linux UAPI posix_acl_xattr: version, then tag/perm/id entries. A named
    // grant masked to zero still constitutes an extended ACL and must be rejected.
    let entries = if default {
        vec![(1u16, 7u16, u32::MAX), (4, 0, u32::MAX), (32, 0, u32::MAX)]
    } else {
        vec![
            (1, 6, u32::MAX),
            (2, 7, 424242),
            (4, 0, u32::MAX),
            (16, 0, u32::MAX),
            (32, 0, u32::MAX),
        ]
    };
    for (tag, perm, id) in entries {
        bytes.extend(tag.to_le_bytes());
        bytes.extend(perm.to_le_bytes());
        bytes.extend(id.to_le_bytes());
    }
    let name = if default {
        c"system.posix_acl_default"
    } else {
        c"system.posix_acl_access"
    };
    // SAFETY: descriptor, name, and byte buffer are valid for this call.
    assert_eq!(
        unsafe {
            libc::fsetxattr(
                file.as_raw_fd(),
                name.as_ptr(),
                bytes.as_ptr().cast(),
                bytes.len(),
                0,
            )
        },
        0,
        "ACL fixture creation failed: {}",
        std::io::Error::last_os_error()
    );
    if !default {
        assert_eq!(file.metadata().unwrap().permissions().mode() & 0o077, 0);
    }
}

#[cfg(target_os = "linux")]
#[test]
fn masked_linux_access_acl_cannot_bypass_private_mode_checks() {
    let fixture = Fixture::new();
    let path = fixture.path().join("store.lock");
    fs::write(&path, "original").unwrap();
    set_linux_acl(&path, false);
    assert!(permissions::validate_path(&path, true).is_err());
    assert!(storage::lock_exclusive(&path).is_err());
    assert!(storage::atomic_replace(&path, b"replacement", true).is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), "original");
}

#[cfg(target_os = "linux")]
#[test]
fn linux_default_acl_is_rejected_before_child_creation_or_package_mutation() {
    let fixture = Fixture::new();
    let store = fixture.path().join("store");
    fs::create_dir(&store).unwrap();
    set_linux_acl(&store, true);
    assert!(storage::lock_exclusive(&store.join("store.lock")).is_err());
    assert!(storage::atomic_replace(&store.join("private"), b"secret", true).is_err());
    assert!(storage::lock_exclusive(&store.join("nested/store.lock")).is_err());
    assert_eq!(fs::read_dir(&store).unwrap().count(), 0);
    let staging = tempfile::Builder::new().tempdir_in(&store).unwrap();
    fs::write(staging.path().join("note"), "fixture").unwrap();
    assert!(storage::atomic_publish_directory(staging, &store.join("package")).is_err());
    assert!(!store.join("package").exists());
}

#[cfg(target_os = "macos")]
#[test]
fn macos_extended_and_inherited_acl_grants_are_rejected_but_denies_are_safe() {
    use std::{os::unix::fs::PermissionsExt, process::Command};
    let fixture = Fixture::new();
    let path = fixture.path().join("store.lock");
    fs::write(&path, "").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(
        Command::new("chmod")
            .arg("+a")
            .arg("everyone deny delete")
            .arg(&path)
            .status()
            .unwrap()
            .success()
    );
    assert!(permissions::validate_path(&path, true).is_ok());
    assert!(
        Command::new("chmod")
            .arg("+a")
            .arg("everyone allow read")
            .arg(&path)
            .status()
            .unwrap()
            .success()
    );
    assert!(permissions::validate_path(&path, true).is_err());
    assert!(storage::lock_exclusive(&path).is_err());
    assert!(
        Command::new("chmod")
            .arg("+a")
            .arg("everyone allow read,write,file_inherit,directory_inherit")
            .arg(fixture.path())
            .status()
            .unwrap()
            .success()
    );
    assert!(storage::atomic_replace(&fixture.path().join("private"), b"secret", true).is_err());
    assert!(!fixture.path().join("private").exists());
}

#[cfg(windows)]
#[test]
fn windows_acl_grants_reject_shared_directory_and_private_file_access() {
    use std::process::Command;
    let fixture = Fixture::new();
    let path = fixture.path().join("store.lock");
    drop(storage::lock_exclusive(&path).unwrap());
    assert!(
        Command::new("icacls")
            .arg(&path)
            .args(["/grant", "*S-1-1-0:(R)"])
            .status()
            .unwrap()
            .success()
    );
    assert!(permissions::validate_path(&path, true).is_err());
    assert!(storage::lock_exclusive(&path).is_err());
    assert!(
        Command::new("icacls")
            .arg(fixture.path())
            .args(["/grant", "*S-1-1-0:(OI)(CI)(W)"])
            .status()
            .unwrap()
            .success()
    );
    assert!(storage::atomic_replace(&fixture.path().join("private"), b"secret", true).is_err());
    assert!(storage::lock_exclusive(&fixture.path().join("nested/store.lock")).is_err());
    assert!(!fixture.path().join("private").exists());
    assert!(!fixture.path().join("nested").exists());
}

#[cfg(windows)]
#[test]
fn windows_junction_is_rejected_before_store_creation() {
    use std::process::Command;
    let fixture = Fixture::new();
    let outside = Fixture::new();
    let junction = fixture.path().join("junction");
    assert!(
        Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&junction)
            .arg(outside.path())
            .status()
            .unwrap()
            .success()
    );
    assert!(storage::lock_exclusive(&junction.join("store.lock")).is_err());
    assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
}

#[cfg(windows)]
#[test]
fn windows_null_dacl_is_rejected_instead_of_treated_as_private() {
    use std::os::windows::{fs::OpenOptionsExt, io::AsRawHandle};
    use windows_sys::Win32::{
        Foundation::GENERIC_READ,
        Security::{
            Authorization::{SE_FILE_OBJECT, SetSecurityInfo},
            DACL_SECURITY_INFORMATION,
        },
        Storage::FileSystem::{READ_CONTROL, WRITE_DAC},
    };
    let fixture = Fixture::new();
    let path = fixture.path().join("store.lock");
    fs::write(&path, "original").unwrap();
    let file = fs::OpenOptions::new()
        .access_mode(GENERIC_READ | READ_CONTROL | WRITE_DAC)
        .open(&path)
        .unwrap();
    // A NULL DACL grants everyone full access; it is NOT an empty deny-all ACL.
    assert_eq!(
        unsafe {
            SetSecurityInfo(
                file.as_raw_handle(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        },
        0
    );
    assert!(permissions::validate_file(&file, true).is_err());
    assert!(storage::lock_exclusive(&path).is_err());
    assert!(storage::atomic_replace(&path, b"replacement", true).is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), "original");
}
