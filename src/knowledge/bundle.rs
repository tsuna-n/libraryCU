use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

const FORMAT: &str = "librarycu-knowledge";
const MAX_ENTRIES: usize = 4096;
const MAX_DOCUMENT_BYTES: u64 = 256 * 1024;
const MAX_ARCHIVE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_MARKDOWN_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct Entry {
    pub id: String,
    pub markdown: String,
    /// Local source locator only; archives never carry filesystem paths.
    pub path: Option<PathBuf>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Archive {
    format: String,
    version: u32,
    entries: Vec<ArchivedEntry>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArchivedEntry {
    id: String,
    sha256: String,
    markdown: String,
}

pub fn digest(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

pub fn validate(entries: &[Entry]) -> Result<()> {
    ensure!(
        entries.len() <= MAX_ENTRIES,
        "knowledge exceeds {MAX_ENTRIES} entries"
    );
    let mut ids = HashSet::new();
    let mut total = 0usize;
    for entry in entries {
        super::loader::validate_id(&entry.id)?;
        ensure!(
            ids.insert(entry.id.to_ascii_lowercase()),
            "duplicate knowledge ID or case-insensitive filename collision"
        );
        total = total
            .checked_add(entry.markdown.len())
            .context("knowledge size overflow")?;
        ensure!(
            total <= MAX_MARKDOWN_BYTES,
            "knowledge exceeds 16 MiB of Markdown"
        );
        ensure!(
            !entry.markdown.contains('\0'),
            "knowledge contains NUL bytes"
        );
        let parsed = super::loader::parse_document("knowledge.md", &entry.markdown)?;
        ensure!(
            parsed.metadata.id == entry.id,
            "archive ID does not match Markdown metadata"
        );
    }
    Ok(())
}

/// Export only authored project notes, or only the personal note store.
pub fn collect(project: &Path, user: bool) -> Result<Vec<Entry>> {
    let roots = if user {
        vec![super::storage::notes_dir()]
    } else {
        crate::security::files::reject_symlinks(project)?;
        ensure!(project.is_dir(), "project directory does not exist");
        vec![project.join("knowledge"), project.join(".lbc/knowledge")]
    };
    let mut entries = Vec::new();
    for root in roots {
        crate::security::files::reject_symlinks(&root)?;
        match fs::symlink_metadata(&root) {
            Ok(metadata) => {
                ensure!(metadata.is_dir(), "knowledge source must be a directory");
                collect_directory(&root, &mut entries, false)?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error).context("failed to inspect knowledge source"),
        }
    }
    validate(&entries)?;
    Ok(entries)
}

pub fn read(path: &Path) -> Result<Vec<Entry>> {
    crate::security::files::reject_symlinks(path)?;
    let resolved = path
        .canonicalize()
        .context("failed to inspect knowledge import")?;
    let metadata = fs::symlink_metadata(path).context("failed to inspect knowledge import")?;
    if metadata.is_dir() {
        let selected_store = selected_knowledge_store(&resolved)?;
        let mut entries = Vec::new();
        collect_directory(path, &mut entries, !selected_store)?;
        validate(&entries)?;
        return Ok(entries);
    }
    let text = crate::security::files::read_text(path, MAX_ARCHIVE_BYTES)?;
    if path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
        && !text.trim_start().starts_with('{')
    {
        selected_knowledge_store(&resolved)?;
        let entry = parse_markdown(path, text)?;
        validate(std::slice::from_ref(&entry))?;
        return Ok(vec![entry]);
    }
    // Checksums detect corruption; they do not authenticate the sender.
    let archive: Archive = serde_json::from_str(&text)
        .map_err(|_| anyhow::anyhow!("invalid knowledge bundle JSON"))?;
    ensure!(
        archive.format == FORMAT,
        "unsupported knowledge bundle format"
    );
    ensure!(archive.version == 1, "unsupported knowledge bundle version");
    ensure!(
        archive.entries.len() <= MAX_ENTRIES,
        "knowledge exceeds {MAX_ENTRIES} entries"
    );
    let mut entries = Vec::with_capacity(archive.entries.len());
    for entry in archive.entries {
        ensure!(
            entry.sha256 == digest(&entry.markdown),
            "knowledge checksum mismatch"
        );
        entries.push(Entry {
            id: entry.id,
            markdown: entry.markdown,
            path: None,
        });
    }
    validate(&entries)?;
    Ok(entries)
}

pub fn write(entries: &[Entry], output: &Path) -> Result<()> {
    validate(entries)?;
    let archive = Archive {
        format: FORMAT.into(),
        version: 1,
        entries: entries
            .iter()
            .map(|entry| ArchivedEntry {
                id: entry.id.clone(),
                sha256: digest(&entry.markdown),
                markdown: entry.markdown.clone(),
            })
            .collect(),
    };
    let bytes = serde_json::to_vec_pretty(&archive)?;
    ensure!(
        bytes.len() as u64 <= MAX_ARCHIVE_BYTES,
        "knowledge bundle exceeds 32 MiB"
    );
    crate::security::storage::atomic_create(output, &bytes, true)
}

fn excluded_directory(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case(".git") || name.eq_ignore_ascii_case(".lbc"))
}

/// An exact knowledge store owns all of its descendants, including hidden notes.
fn selected_knowledge_store(path: &Path) -> Result<bool> {
    let mut components = path.components();
    while let Some(component) = components.next() {
        if component.as_os_str().eq_ignore_ascii_case(".lbc") {
            ensure!(
                components
                    .next()
                    .is_some_and(|next| next.as_os_str().eq_ignore_ascii_case("knowledge")),
                "import knowledge from .lbc/knowledge, not private application directories"
            );
            return Ok(true);
        }
        ensure!(
            component.as_os_str() != ".git",
            "Git directories cannot be imported as knowledge"
        );
    }
    Ok(path
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("knowledge")))
}

fn collect_directory(
    root: &Path,
    entries: &mut Vec<Entry>,
    prune_application_dirs: bool,
) -> Result<()> {
    crate::security::files::reject_symlinks(root)?;
    let mut total: usize = entries.iter().map(|entry| entry.markdown.len()).sum();
    for result in WalkDir::new(root)
        .follow_links(false)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|entry| {
            !prune_application_dirs
                || entry.depth() == 0
                || !entry.file_type().is_dir()
                || !excluded_directory(entry.path())
        })
    {
        let entry = result.context("failed to walk knowledge directory")?;
        ensure!(
            !entry.file_type().is_symlink(),
            "knowledge source contains a symlink"
        );
        if entry.depth() == 0
            || !entry
                .path()
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
        {
            continue;
        }
        ensure!(
            entry.file_type().is_file(),
            "Markdown source is not a regular file"
        );
        ensure!(
            entries.len() < MAX_ENTRIES,
            "knowledge exceeds {MAX_ENTRIES} entries"
        );
        let text = crate::security::files::read_text(entry.path(), MAX_DOCUMENT_BYTES)?;
        total += text.len();
        ensure!(
            total <= MAX_MARKDOWN_BYTES,
            "knowledge exceeds 16 MiB of Markdown"
        );
        entries.push(parse_markdown(entry.path(), text)?);
    }
    Ok(())
}

fn parse_markdown(path: &Path, markdown: String) -> Result<Entry> {
    ensure!(!markdown.contains('\0'), "knowledge contains NUL bytes");
    let document = super::loader::parse_document("knowledge.md", &markdown)?;
    Ok(Entry {
        id: document.metadata.id,
        markdown,
        path: Some(path.to_path_buf()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn entry(id: &str) -> Entry {
        Entry {
            id: id.into(),
            markdown: format!("---\nid: {id}\n---\n# A note\n"),
            path: None,
        }
    }

    #[test]
    fn roundtrip_preserves_older_metadata_and_crlf_exactly() -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
        let path = root.path().join("notes.json");
        let raw = "---\r\nid: old-note\r\nlegacy_field: [keep, this]\r\ntitle_th: เก่า\r\n---\r\n# Old\r\n\r\nBody  \r\n";
        let entries = vec![Entry {
            id: "old-note".into(),
            markdown: raw.into(),
            path: Some(root.path().join("original.md")),
        }];
        write(&entries, &path)?;
        assert_eq!(read(&path)?[0].markdown, raw);
        let encoded = fs::read_to_string(&path)?;
        assert!(!encoded.contains(&root.path().display().to_string()));
        Ok(())
    }

    #[test]
    fn rejects_tampered_checksums_bad_format_versions_and_json() -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
        let path = root.path().join("notes.json");
        write(&[entry("test")], &path)?;
        let valid: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path)?)?;
        for (field, value) in [
            ("format", serde_json::json!("other")),
            ("version", serde_json::json!(2)),
        ] {
            let mut data = valid.clone();
            data[field] = value;
            fs::write(&path, serde_json::to_vec(&data)?)?;
            assert!(read(&path).is_err());
        }
        let mut data = valid.clone();
        data["entries"][0]["markdown"] = serde_json::json!(entry("changed").markdown);
        fs::write(&path, serde_json::to_vec(&data)?)?;
        assert!(read(&path).is_err());
        fs::write(&path, "{broken")?;
        assert!(read(&path).is_err());
        Ok(())
    }

    #[test]
    fn rejects_duplicate_ids_case_collisions_and_mismatched_metadata() {
        assert!(validate(&[entry("same"), entry("same")]).is_err());
        assert!(validate(&[entry("Same"), entry("same")]).is_err());
        let mut note = entry("other");
        note.id = "wrong".into();
        assert!(validate(&[note]).is_err());
        let mut note = entry("bad");
        note.markdown.push('\0');
        assert!(validate(&[note]).is_err());
        assert!(validate(&[entry("../escape")]).is_err());
    }

    #[test]
    fn refuses_missing_invalid_utf8_bad_yaml_and_large_inputs() -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
        let path = root.path().join("note.md");
        assert!(read(&path).is_err());
        for content in [
            b"\xff".as_slice(),
            b"---\nid: [bad\n---\nbody",
            b"body without metadata",
        ] {
            fs::write(&path, content)?;
            assert!(read(&path).is_err());
        }
        fs::File::create(&path)?.set_len(MAX_DOCUMENT_BYTES + 1)?;
        assert!(read(&path).is_err());
        let archive = root.path().join("large.json");
        fs::File::create(&archive)?.set_len(MAX_ARCHIVE_BYTES + 1)?;
        assert!(read(&archive).is_err());
        assert!(validate(&vec![entry("repeated"); MAX_ENTRIES + 1]).is_err());
        let entries: Vec<_> = (0..65)
            .map(|i| Entry {
                id: format!("note-{i}"),
                markdown: format!("---\nid: note-{i}\n---\n{}", "x".repeat(260_000)),
                path: None,
            })
            .collect();
        assert!(validate(&entries).is_err());
        Ok(())
    }

    #[test]
    fn reads_legacy_files_and_directories_and_excludes_runtime() -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
        fs::create_dir(root.path().join("nested"))?;
        fs::write(root.path().join("nested/note.md"), entry("nested").markdown)?;
        fs::write(root.path().join("README.txt"), "not knowledge")?;
        for directory in [".git", ".lbc"] {
            fs::create_dir(root.path().join(directory))?;
            fs::write(
                root.path().join(directory).join("secret.md"),
                "not valid knowledge",
            )?;
        }
        for directory in [".hidden", "runtime"] {
            fs::create_dir(root.path().join(directory))?;
            fs::write(
                root.path().join(directory).join("note.md"),
                entry(&directory.replace('.', "")).markdown,
            )?;
        }
        let entries = read(root.path())?;
        let ids: HashSet<_> = entries.iter().map(|entry| entry.id.as_str()).collect();
        assert_eq!(ids, HashSet::from(["hidden", "nested", "runtime"]));
        assert_eq!(read(&root.path().join("nested/note.md"))?[0].id, "nested");
        assert!(read(&root.path().join(".lbc")).is_err());
        assert_eq!(read(&root.path().join("runtime"))?[0].id, "runtime");
        Ok(())
    }

    #[test]
    fn collection_uses_only_project_knowledge_and_fails_on_corruption() -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
        for directory in [
            "knowledge",
            ".lbc/knowledge",
            ".lbc/runtime",
            ".lbc/packages",
        ] {
            fs::create_dir_all(root.path().join(directory))?;
        }
        fs::write(root.path().join("knowledge/old.md"), entry("old").markdown)?;
        fs::write(
            root.path().join(".lbc/knowledge/new.md"),
            entry("new").markdown,
        )?;
        fs::write(root.path().join(".lbc/runtime/key.md"), "private")?;
        fs::write(root.path().join(".lbc/packages/package.md"), "private")?;
        assert_eq!(collect(root.path(), false)?.len(), 2);
        fs::write(root.path().join("knowledge/bad.md"), "broken")?;
        assert!(collect(root.path(), false).is_err());
        Ok(())
    }

    #[test]
    fn export_covers_every_note_in_selected_stores_including_hidden_and_runtime_subdirs()
    -> Result<()> {
        let root = tempfile::tempdir()?;
        for (index, directory) in [
            "knowledge/.archive",
            "knowledge/runtime",
            ".lbc/knowledge/.archive",
            ".lbc/knowledge/runtime",
            ".lbc/knowledge/.lbc/fixes",
        ]
        .iter()
        .enumerate()
        {
            let directory = root.path().join(directory);
            fs::create_dir_all(&directory)?;
            fs::write(
                directory.join("note.md"),
                entry(&format!("note-{index}")).markdown,
            )?;
        }
        let exported = collect(root.path(), false)?;
        let report = super::super::loader::load_documents_with_data_dir(
            root.path(),
            &root.path().join("missing-packages"),
        )?;
        let loaded_ids: HashSet<_> = report
            .iter()
            .filter(|note| note.source == "project")
            .map(|note| note.metadata.id.as_str())
            .collect();
        let exported_ids: HashSet<_> = exported.iter().map(|note| note.id.as_str()).collect();
        assert_eq!(exported_ids, loaded_ids);
        assert_eq!(exported.len(), 5);
        assert_eq!(read(&root.path().join(".lbc/knowledge"))?.len(), 3);
        Ok(())
    }

    #[test]
    fn accepts_backup_in_arbitrary_runtime_folder_but_rejects_private_lbc_sources() -> Result<()> {
        let root = tempfile::tempdir()?;
        let runtime = root.path().join("runtime");
        fs::create_dir(&runtime)?;
        let backup = runtime.join("backup.json");
        write(&[entry("saved")], &backup)?;
        assert_eq!(read(&backup)?[0].id, "saved");
        let fixes = root.path().join(".lbc/fixes");
        fs::create_dir_all(&fixes)?;
        fs::write(fixes.join("private.md"), entry("private").markdown)?;
        assert!(read(&fixes).is_err());
        assert!(read(&fixes.join("private.md")).is_err());
        let notes = root.path().join(".lbc/knowledge/runtime");
        fs::create_dir_all(&notes)?;
        fs::write(notes.join("valid.md"), entry("valid").markdown)?;
        assert_eq!(read(&notes.join("valid.md"))?[0].id, "valid");
        assert!(read(&notes.join("../../fixes/private.md")).is_err());
        Ok(())
    }

    #[test]
    fn versioned_bundles_roundtrip_inside_lbc_backup_directory() -> Result<()> {
        let root = tempfile::tempdir()?;
        let path = root.path().join(".lbc/backups/snapshot.json");
        write(&[entry("saved")], &path)?;
        assert_eq!(read(&path)?[0].markdown, entry("saved").markdown);
        fs::write(
            root.path().join(".lbc/backups/keys.json"),
            r#"{"key":"private"}"#,
        )?;
        assert!(read(&root.path().join(".lbc/backups/keys.json")).is_err());
        assert!(read(&root.path().join(".lbc/backups")).is_err());
        Ok(())
    }

    #[test]
    fn write_never_overwrites_an_existing_file() -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
        let path = root.path().join("archive");
        fs::write(&path, "keep")?;
        assert!(write(&[entry("note")], &path).is_err());
        assert_eq!(fs::read_to_string(path)?, "keep");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_sources_and_nonregular_markdown_and_writes_privately() -> anyhow::Result<()>
    {
        use std::os::unix::{
            ffi::OsStrExt,
            fs::{PermissionsExt, symlink},
        };
        let root = tempfile::tempdir()?;
        let notes = root.path().join("notes");
        fs::create_dir(&notes)?;
        fs::write(notes.join("note.md"), entry("note").markdown)?;
        symlink(&notes, root.path().join("linked"))?;
        assert!(read(&root.path().join("linked")).is_err());
        symlink(notes.join("note.md"), notes.join("linked.md"))?;
        assert!(read(&notes).is_err());
        fs::remove_file(notes.join("linked.md"))?;
        let fifo = notes.join("fifo.md");
        let name = std::ffi::CString::new(fifo.as_os_str().as_bytes())?;
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        assert!(read(&notes).is_err());
        let output = root.path().join("bundle.json");
        write(&[entry("note")], &output)?;
        assert_eq!(fs::metadata(output)?.permissions().mode() & 0o777, 0o600);
        Ok(())
    }
}
