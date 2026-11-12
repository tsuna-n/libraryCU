use std::{
    collections::HashSet,
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, bail};

use super::loader::{parse_document_for_source, validate_id};
use super::{KnowledgeDocument, KnowledgeMetadata, load_all_documents};

const MAX_NOTE_BYTES: u64 = 256 * 1024;

pub struct AddEntry<'a> {
    pub id: Option<&'a str>,
    pub title: &'a str,
    pub kind: &'a str,
    pub body: &'a str,
    pub project: Option<&'a Path>,
    pub overrides: Option<&'a str>,
}
pub struct EditEntry<'a> {
    pub reference: &'a str,
    pub replacement: Option<&'a str>,
    pub project: &'a Path,
    pub create_override: bool,
}

pub fn notes_dir() -> PathBuf {
    super::packages::user_data_dir().join("notes")
}

pub fn add_entry(input: AddEntry<'_>) -> Result<KnowledgeDocument> {
    add_entry_impl(input, None)
}

/// Capture structured metadata through the same locked, non-overwriting writer as `add`.
pub fn add_entry_with_metadata(
    input: AddEntry<'_>,
    metadata: KnowledgeMetadata,
) -> Result<KnowledgeDocument> {
    add_entry_impl(input, Some(metadata))
}

fn add_entry_impl(
    input: AddEntry<'_>,
    metadata: Option<KnowledgeMetadata>,
) -> Result<KnowledgeDocument> {
    if input.title.trim().is_empty() {
        bail!("title must not be empty");
    }
    if input.body.trim().is_empty() {
        bail!("note body must not be empty");
    }
    if input.body.len() as u64 > MAX_NOTE_BYTES {
        bail!("note is larger than 256 KB");
    }
    if !matches!(input.kind, "note" | "concept" | "troubleshooting") {
        bail!("kind must be note, concept, or troubleshooting");
    }
    let (root, anchor) = if let Some(project) = input.project {
        crate::security::files::reject_symlinks(project)?;
        let project = project
            .canonicalize()
            .with_context(|| format!("project path does not exist: {}", project.display()))?;
        (project.join(".lbc/knowledge"), project)
    } else {
        let root = notes_dir();
        let anchor = root
            .parent()
            .and_then(Path::parent)
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        (root, anchor)
    };
    crate::security::files::reject_symlinks(&root)?;
    crate::security::permissions::validate_directory(&root)?;
    if input.project.is_some() {
        crate::security::git::ensure_gitignore(&anchor)?;
    }
    fs::create_dir_all(&root).with_context(|| format!("failed to create {}", root.display()))?;
    ensure_store_is_safe(&root, &anchor)?;
    let _lock = crate::security::storage::lock_exclusive(&store_lock_path(&root))?;
    let base = input
        .id
        .map(str::to_owned)
        .unwrap_or_else(|| slug(input.title));
    validate_id(&base)?;
    let mut ids = stored_ids(&root)?;
    if input.project.is_some() {
        ids.extend(stored_ids(&anchor.join("knowledge"))?);
    }
    let id = if input.id.is_some() {
        if ids.contains(&base) {
            bail!(
                "entry ID {base:?} already exists in this source; use edit or choose a different ID"
            );
        }
        base
    } else {
        unique_id(&root, &base, &ids)
    };
    let target = root.join(format!("{id}.md"));
    if target.exists() {
        bail!(
            "entry {id:?} already exists at {}; choose another ID",
            target.display()
        );
    }
    let encoded = if let Some(mut metadata) = metadata {
        metadata.id = id.clone();
        metadata.title = Some(input.title.to_owned());
        metadata.kind = Some(input.kind.to_owned());
        metadata.overrides = input.overrides.map(str::to_owned);
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        metadata.created_at_unix = Some(now);
        metadata.updated_at_unix = Some(now);
        encode_metadata(&metadata, input.body)?
    } else {
        encode(&id, input.title, input.kind, input.overrides, input.body)?
    };
    let source = if input.project.is_some() {
        "project"
    } else {
        "user"
    };
    if encoded.len() as u64 > MAX_NOTE_BYTES {
        bail!("note including metadata is larger than 256 KB; nothing was saved");
    }
    let document = parse_document_for_source(
        &format!("{id}.md"),
        &encoded,
        source,
        &target.display().to_string(),
        true,
    )?;
    atomic_write_new(&target, encoded.as_bytes())?;
    Ok(document)
}

pub fn inspect_entry(reference: &str, project: &Path) -> Result<KnowledgeDocument> {
    let report = load_all_documents(project)?;
    resolve_entry(&report.documents, reference).cloned()
}

pub fn resolve_entry<'a>(
    documents: &'a [KnowledgeDocument],
    reference: &str,
) -> Result<&'a KnowledgeDocument> {
    let matches: Vec<_> = if reference.contains(':') {
        documents
            .iter()
            .filter(|document| document.source_id == reference)
            .collect()
    } else {
        documents
            .iter()
            .filter(|document| document.metadata.id == reference)
            .collect()
    };
    match matches.as_slice() {
        [] => bail!("knowledge entry {reference:?} was not found"),
        [document] => Ok(document),
        many => bail!(
            "entry ID {reference:?} is ambiguous; use one of: {}",
            many.iter()
                .map(|document| document.source_id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

pub fn edit_entry(input: EditEntry<'_>) -> Result<KnowledgeDocument> {
    edit_entry_impl(input, None)
}

/// Replace only the version the user reviewed; preserve newer edits made during capture.
pub fn edit_entry_from_preview(
    original: &KnowledgeDocument,
    body: &str,
    project: &Path,
) -> Result<KnowledgeDocument> {
    edit_entry_impl(
        EditEntry {
            reference: &original.source_id,
            replacement: Some(body),
            project,
            create_override: false,
        },
        Some(original),
    )
}

fn edit_entry_impl(
    input: EditEntry<'_>,
    reviewed: Option<&KnowledgeDocument>,
) -> Result<KnowledgeDocument> {
    let original = match reviewed {
        Some(original) => original.clone(),
        None => inspect_entry(input.reference, input.project)?,
    };
    if original.source == "builtin" && !input.create_override {
        bail!(
            "{} is built in and read-only; use --override to create a user override",
            original.source_id
        );
    }
    if !original.writable && !input.create_override {
        bail!("{} is read-only", original.source_id);
    }
    let report = if input.create_override {
        Some(load_all_documents(input.project)?)
    } else {
        None
    };
    let existing_override = report.as_ref().and_then(|report| {
        report.documents.iter().find(|document| {
            document.source == "user"
                && document.metadata.overrides.as_deref() == Some(&original.source_id)
        })
    });
    let editing = existing_override.unwrap_or(&original);
    let original_text = if editing.writable {
        Some(crate::security::files::read_text(
            Path::new(&editing.path),
            MAX_NOTE_BYTES,
        )?)
    } else {
        None
    };
    let body = match input.replacement {
        Some(body) => body.to_owned(),
        None => edit_with_editor(&editing.body)?,
    };
    if body.trim().is_empty() {
        bail!("replacement body must not be empty; original entry was preserved");
    }
    if body.len() as u64 > MAX_NOTE_BYTES {
        bail!("replacement is larger than 256 KB; original entry was preserved");
    }
    if input.create_override {
        if let Some(existing) = existing_override {
            let target = PathBuf::from(&existing.path);
            let parent = target.parent().context("entry has no parent directory")?;
            let _lock = crate::security::storage::lock_exclusive(&store_lock_path(parent))?;
            ensure_existing_target_is_safe(&target)?;
            ensure_document_unchanged(existing, input.project)?;
            ensure_text_unchanged(&target, original_text.as_deref())?;
            if body.trim() == existing.body.trim() {
                return inspect_entry(&existing.source_id, input.project);
            }
            let encoded = encode_existing(existing, &body)?;
            parse_document_for_source("override.md", &encoded, "user", &existing.path, true)
                .context("replacement is not a valid knowledge document")?;
            preserve_revision(
                &existing.metadata.id,
                original_text.as_deref().context("missing original note")?,
                input.project,
                true,
            )?;
            ensure_text_unchanged(&target, original_text.as_deref())?;
            atomic_replace(&target, encoded.as_bytes())?;
            return inspect_entry(&existing.source_id, input.project);
        }
        let mut metadata = original.metadata.clone();
        metadata.verification_status = Some("unverified".to_owned());
        return add_entry_with_metadata(
            AddEntry {
                id: Some(&original.metadata.id),
                title: &original.title,
                kind: &original.kind,
                body: &body,
                project: None,
                overrides: Some(&original.source_id),
            },
            metadata,
        );
    }
    let target = PathBuf::from(&original.path);
    let parent = target.parent().context("entry has no parent directory")?;
    let _lock = crate::security::storage::lock_exclusive(&store_lock_path(parent))?;
    ensure_existing_target_is_safe(&target)?;
    ensure_document_unchanged(&original, input.project)?;
    ensure_text_unchanged(&target, original_text.as_deref())?;
    if body.trim() == original.body.trim() {
        return inspect_entry(&original.source_id, input.project);
    }
    let encoded = encode_existing(&original, &body)?;
    parse_document_for_source(
        target
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("entry.md"),
        &encoded,
        &original.source,
        &original.path,
        true,
    )
    .context("replacement is not a valid knowledge document")?;
    preserve_revision(
        &original.metadata.id,
        original_text.as_deref().context("missing original note")?,
        input.project,
        original.source == "user",
    )?;
    ensure_text_unchanged(&target, original_text.as_deref())?;
    atomic_replace(&target, encoded.as_bytes())?;
    inspect_entry(&original.source_id, input.project)
}

fn encode(
    id: &str,
    title: &str,
    kind: &str,
    overrides: Option<&str>,
    body: &str,
) -> Result<String> {
    let mut metadata = serde_yaml::Mapping::new();
    metadata.insert("id".into(), id.into());
    metadata.insert("title".into(), title.into());
    metadata.insert("kind".into(), kind.into());
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    metadata.insert("created_at_unix".into(), now.into());
    metadata.insert("updated_at_unix".into(), now.into());
    if let Some(target) = overrides {
        metadata.insert("overrides".into(), target.into());
    }
    let frontmatter = serde_yaml::to_string(&metadata)?
        .trim_start_matches("---\n")
        .trim_end()
        .to_owned();
    Ok(format!("---\n{frontmatter}\n---\n{}\n", body.trim()))
}

fn encode_existing(document: &KnowledgeDocument, body: &str) -> Result<String> {
    let mut metadata = document.metadata.clone();
    metadata.title = Some(document.title.clone());
    metadata.kind = Some(document.kind.clone());
    metadata.verification_status = Some(if body.trim() == document.body.trim() {
        document.verification_status.clone()
    } else {
        "unverified".to_owned()
    });
    if body.trim() != document.body.trim() {
        metadata.updated_at_unix = Some(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs());
    }
    encode_metadata(&metadata, body)
}

fn encode_metadata(metadata: &KnowledgeMetadata, body: &str) -> Result<String> {
    let mut value = serde_yaml::to_value(metadata)?;
    let mapping = value
        .as_mapping_mut()
        .context("metadata must be a YAML mapping")?;
    for (key, value) in &metadata.extra {
        mapping
            .entry(key.clone().into())
            .or_insert_with(|| value.clone());
    }
    let frontmatter = serde_yaml::to_string(&value)?
        .trim_start_matches("---\n")
        .trim_end()
        .to_owned();
    Ok(format!("---\n{frontmatter}\n---\n{}\n", body.trim()))
}

fn ensure_text_unchanged(target: &Path, original: Option<&str>) -> Result<()> {
    if Some(crate::security::files::read_text(target, MAX_NOTE_BYTES)?.as_str()) != original {
        bail!("knowledge entry changed during editing; the newer content was preserved");
    }
    Ok(())
}

/// Keep exact authored bytes before any replacement; never prune revisions.
pub(crate) fn preserve_revision(
    id: &str,
    markdown: &str,
    project: &Path,
    user: bool,
) -> Result<()> {
    validate_id(id)?;
    let root = if user {
        super::packages::user_data_dir().join("knowledge-history")
    } else {
        crate::security::git::ensure_gitignore(project)?;
        project.join(".lbc/knowledge-history")
    };
    let target = root
        .join(format!("{id}.revisions"))
        .join(format!("{}.md", super::bundle::digest(markdown)));
    crate::security::files::reject_symlinks(&target)?;
    match fs::symlink_metadata(&target) {
        Ok(_) => {
            if crate::security::files::read_store_text(&target, MAX_NOTE_BYTES, true)? != markdown {
                bail!("existing knowledge revision differs; original note was preserved");
            }
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            crate::security::storage::atomic_create(&target, markdown.as_bytes(), true)
                .context("failed to preserve knowledge revision; original note was preserved")
        }
        Err(error) => Err(error).context("failed to inspect knowledge revision"),
    }
}

fn slug(title: &str) -> String {
    let value = title
        .chars()
        .flat_map(char::to_lowercase)
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let value = value
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if value.is_empty() {
        "note".to_owned()
    } else {
        value.chars().take(64).collect()
    }
}
fn unique_id(root: &Path, base: &str, ids: &HashSet<String>) -> String {
    if !root.join(format!("{base}.md")).exists() && !ids.contains(base) {
        return base.to_owned();
    }
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("{base}-{stamp}")
}

fn stored_ids(root: &Path) -> Result<HashSet<String>> {
    crate::security::files::reject_symlinks(root)?;
    let mut ids = HashSet::new();
    if !root.exists() {
        return Ok(ids);
    }
    for entry in walkdir::WalkDir::new(root).follow_links(false) {
        let entry = entry?;
        if !entry.file_type().is_file()
            || entry.path().extension().and_then(|s| s.to_str()) != Some("md")
        {
            continue;
        }
        let content = crate::security::files::read_text(entry.path(), MAX_NOTE_BYTES)?;
        if let Ok(document) =
            super::loader::parse_document(&entry.path().display().to_string(), &content)
        {
            ids.insert(document.metadata.id);
        }
    }
    Ok(ids)
}
fn edit_with_editor(original: &str) -> Result<String> {
    let editor = env::var("VISUAL")
        .or_else(|_| env::var("EDITOR"))
        .context("set VISUAL or EDITOR, or pass --file")?;
    let parts = shell_words::split(&editor).context("invalid quoting in VISUAL or EDITOR")?;
    let program = parts
        .first()
        .filter(|part| !part.is_empty())
        .context("editor command is empty")?;
    let mut temp = tempfile::Builder::new()
        .prefix("lbc-edit-")
        .suffix(".md")
        .tempfile()?;
    crate::security::permissions::validate_file(temp.as_file(), true)?;
    temp.write_all(original.as_bytes())?;
    temp.as_file().sync_all()?;
    let status = Command::new(program)
        .args(&parts[1..])
        .arg(temp.path())
        .status()
        .with_context(|| format!("failed to launch editor {program:?}"))?;
    if !status.success() {
        bail!("editor exited without saving; original entry was preserved");
    }
    crate::security::files::read_text(temp.path(), MAX_NOTE_BYTES)
}
fn atomic_write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let temp = prepared_temp(path, bytes)?;
    crate::security::permissions::validate_directory(
        path.parent().context("entry has no parent")?,
    )?;
    crate::security::permissions::validate_file(temp.as_file(), true)?;
    temp.persist_noclobber(path)
        .with_context(|| format!("refusing to overwrite {}", path.display()))?;
    crate::security::storage::sync_directory(
        path.parent().context("entry has no parent directory")?,
    )?;
    Ok(())
}
pub(crate) fn store_lock_path(store: &Path) -> PathBuf {
    let name = store
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("knowledge");
    store
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!(".{name}.lock"))
}
fn atomic_replace(path: &Path, bytes: &[u8]) -> Result<()> {
    crate::security::storage::atomic_replace(path, bytes, false)
        .with_context(|| format!("failed to replace {}", path.display()))
}
fn prepared_temp(path: &Path, bytes: &[u8]) -> Result<tempfile::NamedTempFile> {
    let parent = path.parent().context("entry has no parent directory")?;
    crate::security::permissions::validate_directory(parent)?;
    let mut temp = tempfile::Builder::new()
        .prefix(".lbc-write-")
        .tempfile_in(parent)?;
    crate::security::permissions::validate_file(temp.as_file(), true)?;
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    Ok(temp)
}
fn ensure_store_is_safe(path: &Path, anchor: &Path) -> Result<()> {
    if fs::symlink_metadata(path)?.file_type().is_symlink() {
        bail!("refusing to use symlinked store {}", path.display());
    }
    let canonical = path.canonicalize()?;
    let canonical_anchor = anchor.canonicalize()?;
    if !canonical.starts_with(&canonical_anchor) {
        bail!(
            "knowledge store {} escapes its selected root",
            path.display()
        );
    }
    Ok(())
}
fn ensure_existing_target_is_safe(path: &Path) -> Result<()> {
    crate::security::files::reject_symlinks(path)?;
    if fs::symlink_metadata(path)?.file_type().is_symlink() {
        bail!("refusing to edit symlink {}", path.display());
    }
    let parent = path
        .parent()
        .context("entry has no parent")?
        .canonicalize()?;
    let canonical = path.canonicalize()?;
    if !canonical.starts_with(&parent) {
        bail!("entry escapes its selected store");
    }
    Ok(())
}
fn ensure_document_unchanged(original: &KnowledgeDocument, project: &Path) -> Result<()> {
    let current = inspect_entry(&original.source_id, project)?;
    if current.path != original.path
        || current.metadata != original.metadata
        || current.body != original.body
        || current.verification_status != original.verification_status
    {
        bail!("knowledge entry changed during editing; the newer content was preserved");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_notes_receive_creation_and_update_dates() -> Result<()> {
        let project = tempfile::tempdir()?;
        let note = add_entry(AddEntry {
            id: Some("dated"),
            title: "Dated",
            kind: "note",
            body: "Original",
            project: Some(project.path()),
            overrides: None,
        })?;
        assert!(note.metadata.created_at_unix.is_some());
        assert_eq!(note.metadata.created_at_unix, note.metadata.updated_at_unix);
        Ok(())
    }

    #[test]
    fn editing_legacy_notes_keeps_ids_custom_metadata_and_exact_revision() -> Result<()> {
        let project = tempfile::tempdir()?;
        let store = project.path().join(".lbc/knowledge");
        fs::create_dir_all(&store)?;
        let path = store.join("legacy.md");
        let raw = "---\r\nid: stable\r\ncustom_field: {owner: team, values: [one, two]}\r\nreferences: [builtin:rust-e0308]\r\n---\r\n# Original\r\n\r\nOriginal body.  \r\n";
        fs::write(&path, raw)?;
        let updated = edit_entry(EditEntry {
            reference: "project:stable",
            replacement: Some("Revised body"),
            project: project.path(),
            create_override: false,
        })?;
        assert_eq!(updated.source_id, "project:stable");
        assert_eq!(updated.path, path.display().to_string());
        assert_eq!(updated.metadata.created_at_unix, None);
        assert!(updated.metadata.updated_at_unix.is_some());
        assert_eq!(updated.metadata.references, ["builtin:rust-e0308"]);
        let saved = fs::read_to_string(&path)?;
        assert!(saved.contains("custom_field:"));
        assert!(saved.contains("owner: team"));
        let history = project.path().join(".lbc/knowledge-history");
        let revisions: Vec<_> = walkdir::WalkDir::new(history)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
            .collect();
        assert_eq!(revisions.len(), 1);
        assert_eq!(fs::read_to_string(revisions[0].path())?, raw);
        let before_noop = fs::read(&path)?;
        edit_entry(EditEntry {
            reference: "project:stable",
            replacement: Some("Revised body"),
            project: project.path(),
            create_override: false,
        })?;
        assert_eq!(fs::read(&path)?, before_noop);
        Ok(())
    }

    #[test]
    fn failed_revision_write_preserves_original_note() -> Result<()> {
        let project = tempfile::tempdir()?;
        let note = add_entry(AddEntry {
            id: Some("protected"),
            title: "Protected",
            kind: "note",
            body: "Keep me",
            project: Some(project.path()),
            overrides: None,
        })?;
        let original = fs::read(&note.path)?;
        fs::write(project.path().join(".lbc/knowledge-history"), "blocked")?;
        let result = edit_entry(EditEntry {
            reference: &note.source_id,
            replacement: Some("Replacement"),
            project: project.path(),
            create_override: false,
        });
        assert!(result.is_err());
        assert_eq!(fs::read(&note.path)?, original);
        Ok(())
    }

    #[test]
    fn captured_edit_preserves_changes_made_after_preview() {
        let project = tempfile::tempdir().unwrap();
        let original = add_entry(AddEntry {
            id: Some("preview-race"),
            title: "Preview race",
            kind: "troubleshooting",
            body: "## Problem\n\nWorker stopped\n\n## Solution\n\nOriginal fix\n",
            project: Some(project.path()),
            overrides: None,
        })
        .unwrap();
        let newer = edit_entry(EditEntry {
            reference: &original.source_id,
            replacement: Some("## Problem\n\nWorker stopped\n\n## Solution\n\nNewer fix\n"),
            project: project.path(),
            create_override: false,
        })
        .unwrap();
        let bytes = fs::read(&newer.path).unwrap();
        let error =
            edit_entry_from_preview(&original, "Stale replacement", project.path()).unwrap_err();
        assert!(error.to_string().contains("newer content was preserved"));
        assert_eq!(fs::read(&newer.path).unwrap(), bytes);
    }

    #[test]
    fn legacy_yaml_metadata_survives_edit_serialization() -> Result<()> {
        let legacy = "---\nid: null\ntitle: 'YAML: quotes, # and ไทย'\nkind: troubleshooting\nlanguage: &language rust\ntool: *language\nframework: null\ncategory: ~\nerror_code: E0308\ntitle_th: แก้ไขชนิดข้อมูล\ntags: [null, 'true', '0123', 'a: b']\nkeywords:\n  - |\n    first line\n    second line\nverification_status: recorded-check\ncreated_at_unix: 18446744073709551615\nupdated_at_unix: 1\n<<: {kind: ignored, title: ignored}\n---\n# Existing note\n\nKeep `exact_code()` and ไทย.\n";
        let original = parse_document_for_source("legacy.md", legacy, "user", "/legacy.md", true)?;
        assert_eq!(original.metadata.id, "null");
        assert_eq!(original.metadata.tool.as_deref(), Some("rust"));
        assert_eq!(original.metadata.framework, None);
        assert_eq!(original.metadata.category, None);
        assert_eq!(original.metadata.tags, ["null", "true", "0123", "a: b"]);
        assert_eq!(original.metadata.created_at_unix, Some(u64::MAX));
        assert_eq!(original.metadata.keywords, ["first line\nsecond line\n"]);

        let encoded = encode_existing(&original, &original.body)?;
        let reloaded =
            parse_document_for_source("legacy.md", &encoded, "user", "/legacy.md", true)?;
        assert_eq!(reloaded.metadata, original.metadata);
        assert_eq!(reloaded.body, original.body);
        assert!(encoded.starts_with("---\n"));
        Ok(())
    }

    #[test]
    fn arbitrary_custom_yaml_survives_edits_without_breaking_json() -> Result<()> {
        let raw = "---\nid: custom-yaml\ncustom: {[a, b]: value}\n---\nOriginal body\n";
        let original = parse_document_for_source("custom.md", raw, "user", "/custom.md", true)?;
        serde_json::to_string(&original)?;
        let encoded = encode_existing(&original, "Updated body")?;
        let reloaded =
            parse_document_for_source("custom.md", &encoded, "user", "/custom.md", true)?;
        assert_eq!(original.metadata.extra, reloaded.metadata.extra);
        serde_json::to_string(&reloaded)?;
        Ok(())
    }

    #[test]
    fn yaml_capture_roundtrips_special_and_multiline_titles() -> Result<()> {
        for title in [
            "null",
            "true",
            "0123",
            "a: b # ไทย",
            "first line\n---\nsecond line",
        ] {
            let encoded = encode(
                "yaml-note",
                title,
                "note",
                Some("builtin:rust-e0308"),
                "Keep body",
            )?;
            let document =
                parse_document_for_source("note.md", &encoded, "user", "/note.md", true)?;
            assert_eq!(document.title, title);
            assert_eq!(
                document.metadata.overrides.as_deref(),
                Some("builtin:rust-e0308")
            );
            assert_eq!(document.body, "Keep body");
        }
        Ok(())
    }
}
