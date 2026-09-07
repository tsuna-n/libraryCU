//! Local recovery records are published before any source mutation. They are
//! not a transaction log for arbitrary commands or a defense against local tampering.
use super::*;

const MAX_RECORD_BYTES: u64 = MAX_SOURCE_BYTES * 12 + 8192;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    version: u32,
    root: PathBuf,
    path: String,
    original: String,
    applied: String,
}

pub fn prepare(target: &Target, patch: &Patch) -> Result<String> {
    target.check_unchanged()?;
    target.validate_response(&serde_json::to_string(&Replacement {
        before: patch.before.clone(),
        after: patch.after.clone(),
    })?)?;
    ensure!(patch.path == target.relative, "patch target mismatch");
    let directory = target.root.join(".lbc/fixes");
    security::files::reject_symlinks(&directory)?;
    fs::create_dir_all(&directory)?;
    security::files::reject_symlinks(&directory)?;
    let record = Record {
        version: 1,
        root: target.root.clone(),
        path: target.relative.clone(),
        original: target.original.clone(),
        applied: target.original.replacen(&patch.before, &patch.after, 1),
    };
    let mut temporary = tempfile::Builder::new()
        .prefix("fix-")
        .tempfile_in(&directory)?;
    let id = temporary
        .path()
        .file_name()
        .context("missing recovery ID")?
        .to_str()
        .context("invalid recovery ID")?
        .to_owned();
    let content = serde_json::to_vec(&record)?;
    ensure!(
        content.len() as u64 <= MAX_RECORD_BYTES,
        "recovery record is too large"
    );
    temporary.write_all(&content)?;
    temporary.as_file().sync_all()?;
    temporary
        .persist_noclobber(directory.join(format!("{id}.json")))
        .context("cannot publish recovery record")?;
    Ok(id)
}

pub fn restore(root: &Path, id: &str) -> Result<String> {
    ensure!(
        id.starts_with("fix-")
            && id.len() <= 100
            && id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_'),
        "invalid recovery ID"
    );
    security::files::reject_symlinks(root)?;
    let root = root.canonicalize()?;
    let content = security::files::read_text(
        &root.join(".lbc/fixes").join(format!("{id}.json")),
        MAX_RECORD_BYTES,
    )?;
    let record: Record = serde_json::from_str(&content).context("invalid recovery record")?;
    ensure!(
        record.version == 1 && record.root == root,
        "recovery record belongs to a different project or version"
    );
    let relative = Path::new(&record.path);
    ensure!(
        !relative.as_os_str().is_empty()
            && relative
                .components()
                .all(|c| matches!(c, Component::Normal(_))),
        "unsafe recovery target"
    );
    ensure!(
        !relative
            .components()
            .any(|c| crate::scanner::ignore::should_ignore_directory(
                Path::new(c.as_os_str()),
                true
            )),
        "hidden or generated recovery target"
    );
    ensure!(
        record.original.len() as u64 <= MAX_SOURCE_BYTES
            && record.applied.len() as u64 <= MAX_SOURCE_BYTES,
        "recovery source too large"
    );
    ensure!(
        security::redact_sensitive(&record.original) == record.original
            && security::redact_sensitive(&record.applied) == record.applied,
        "recovery source contains recognizable secrets"
    );
    let path = root.join(relative);
    let check = || -> Result<()> {
        let current = security::files::read_text(&path, MAX_SOURCE_BYTES)?;
        ensure!(
            path.canonicalize()?.starts_with(&root),
            "recovery target escaped project"
        );
        ensure!(
            current == record.applied,
            "rollback refused: source changed after application; preserve your edits and recover manually"
        );
        ensure!(
            !fs::metadata(&path)?.permissions().readonly(),
            "recovery target is read-only"
        );
        Ok(())
    };
    check()?;
    let permissions = fs::metadata(&path)?.permissions();
    let mut temporary = tempfile::NamedTempFile::new_in(path.parent().context("missing parent")?)?;
    temporary.write_all(record.original.as_bytes())?;
    temporary.as_file().set_permissions(permissions)?;
    temporary.as_file().sync_all()?;
    check()?;
    temporary
        .persist(&path)
        .context("cannot publish rollback atomically")?;
    Ok(record.path)
}
