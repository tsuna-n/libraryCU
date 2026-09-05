use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, bail};

use super::loader::{parse_document_for_source, validate_id};
use super::{KnowledgeDocument, load_all_documents};

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
    if let Some(path) = env::var_os("XDG_DATA_HOME") {
        PathBuf::from(path).join("lbc/notes")
    } else if let Some(home) = env::var_os("HOME") {
        PathBuf::from(home).join(".local/share/lbc/notes")
    } else {
        PathBuf::from(".local/share/lbc/notes")
    }
}

pub fn add_entry(input: AddEntry<'_>) -> Result<KnowledgeDocument> {
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
    fs::create_dir_all(&root).with_context(|| format!("failed to create {}", root.display()))?;
    ensure_store_is_safe(&root, &anchor)?;
    let base = input
        .id
        .map(str::to_owned)
        .unwrap_or_else(|| slug(input.title));
    validate_id(&base)?;
    let id = if input.id.is_some() {
        base
    } else {
        unique_id(&root, &base)
    };
    let target = root.join(format!("{id}.md"));
    if target.exists() {
        bail!(
            "entry {id:?} already exists at {}; choose another ID",
            target.display()
        );
    }
    let encoded = encode(&id, input.title, input.kind, input.overrides, input.body)?;
    let source = if input.project.is_some() {
        "project"
    } else {
        "user"
    };
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
    let original = inspect_entry(input.reference, input.project)?;
    if original.source == "builtin" && !input.create_override {
        bail!(
            "{} is built in and read-only; use --override to create a user override",
            original.source_id
        );
    }
    if !original.writable && !input.create_override {
        bail!("{} is read-only", original.source_id);
    }
    let body = match input.replacement {
        Some(body) => body.to_owned(),
        None => edit_with_editor(&original.body)?,
    };
    if body.trim().is_empty() {
        bail!("replacement body must not be empty; original entry was preserved");
    }
    if body.len() as u64 > MAX_NOTE_BYTES {
        bail!("replacement is larger than 256 KB; original entry was preserved");
    }
    if input.create_override {
        let report = load_all_documents(input.project)?;
        if let Some(existing) = report.documents.iter().find(|document| {
            document.source == "user"
                && document.metadata.overrides.as_deref() == Some(&original.source_id)
        }) {
            let target = PathBuf::from(&existing.path);
            ensure_existing_target_is_safe(&target)?;
            let encoded = encode_existing(existing, &body)?;
            parse_document_for_source("override.md", &encoded, "user", &existing.path, true)
                .context("replacement is not a valid knowledge document")?;
            atomic_replace(&target, encoded.as_bytes())?;
            return inspect_entry(&existing.source_id, input.project);
        }
        return add_entry(AddEntry {
            id: Some(&original.metadata.id),
            title: &original.title,
            kind: &original.kind,
            body: &body,
            project: None,
            overrides: Some(&original.source_id),
        });
    }
    let target = PathBuf::from(&original.path);
    ensure_existing_target_is_safe(&target)?;
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
    metadata.verification_status = Some(document.verification_status.clone());
    let frontmatter = serde_yaml::to_string(&metadata)?
        .trim_start_matches("---\n")
        .trim_end()
        .to_owned();
    Ok(format!("---\n{frontmatter}\n---\n{}\n", body.trim()))
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
fn unique_id(root: &Path, base: &str) -> String {
    if !root.join(format!("{base}.md")).exists() {
        return base.to_owned();
    }
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    format!("{base}-{stamp}")
}
fn edit_with_editor(original: &str) -> Result<String> {
    let editor = env::var("VISUAL")
        .or_else(|_| env::var("EDITOR"))
        .context("set VISUAL or EDITOR, or pass --file")?;
    let mut parts = editor.split_whitespace();
    let program = parts
        .next()
        .filter(|part| !part.is_empty())
        .context("editor command is empty")?;
    let temp = env::temp_dir().join(format!(
        "lbc-edit-{}-{}.md",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default()
    ));
    atomic_write_new(&temp, original.as_bytes())?;
    let status = Command::new(program)
        .args(parts)
        .arg(&temp)
        .status()
        .with_context(|| format!("failed to launch editor {program:?}"))?;
    if !status.success() {
        let _ = fs::remove_file(&temp);
        bail!("editor exited without saving; original entry was preserved");
    }
    let body = fs::read_to_string(&temp)?;
    let _ = fs::remove_file(&temp);
    Ok(body)
}
fn atomic_write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::fs::OpenOptions;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .with_context(|| format!("refusing to overwrite {}", path.display()))?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}
fn atomic_replace(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().context("entry has no parent directory")?;
    let temp = parent.join(format!(
        ".lbc-edit-{}-{}.tmp",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default()
    ));
    atomic_write_new(&temp, bytes)?;
    fs::rename(&temp, path).with_context(|| format!("failed to replace {}", path.display()))?;
    Ok(())
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
