use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, ensure};
use serde::Serialize;

use super::bundle::Entry;

#[derive(Debug, Default, Serialize)]
pub struct TransferReport {
    pub added: Vec<String>,
    pub replaced: Vec<String>,
    pub duplicates: Vec<String>,
    pub similar: Vec<String>,
    pub conflicts: Vec<String>,
}

pub struct Plan {
    project: PathBuf,
    user: bool,
    incoming: Vec<Entry>,
    existing: Vec<Entry>,
    pub report: TransferReport,
}

pub fn prepare(project: &Path, user: bool, incoming: Vec<Entry>) -> Result<Plan> {
    super::bundle::validate(&incoming)?;
    let existing = super::bundle::collect(project, user)?;
    let source = if user { "user" } else { "project" };
    let mut documents = existing
        .iter()
        .map(|entry| {
            super::loader::parse_document_for_source(
                &format!("{}.md", entry.id),
                &entry.markdown,
                source,
                "",
                true,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let mut report = TransferReport::default();
    for entry in &incoming {
        if let Some(current) = existing
            .iter()
            .find(|current| current.id.eq_ignore_ascii_case(&entry.id))
        {
            ensure!(
                current.id == entry.id,
                "portable ID conflict for {}; nothing was written",
                entry.id
            );
            if current.markdown == entry.markdown {
                report.duplicates.push(entry.id.clone());
            } else {
                report.conflicts.push(entry.id.clone());
                report.replaced.push(entry.id.clone());
            }
        } else {
            let document = super::loader::parse_document_for_source(
                &format!("{}.md", entry.id),
                &entry.markdown,
                source,
                "",
                true,
            )?;
            let problem = document
                .troubleshooting_details()
                .map(|details| details.problem)
                .unwrap_or_else(|| document.body.clone());
            for similar in super::duplicates::find_similar(
                &documents,
                &document.title,
                &problem,
                &document.metadata,
            ) {
                report
                    .similar
                    .push(format!("{} resembles {}", entry.id, similar.source_id));
            }
            documents.push(document);
            report.added.push(entry.id.clone());
        }
    }
    Ok(Plan {
        project: project.to_path_buf(),
        user,
        incoming,
        existing,
        report,
    })
}

pub fn apply(plan: Plan, replace: bool) -> Result<TransferReport> {
    apply_with_hook(plan, replace, || Ok(()))
}

fn apply_with_hook(
    plan: Plan,
    replace: bool,
    mut after_write: impl FnMut() -> Result<()>,
) -> Result<TransferReport> {
    ensure!(
        replace || plan.report.conflicts.is_empty(),
        "restore/import conflicts require --replace and confirmation; nothing was written"
    );
    if plan.report.added.is_empty() && plan.report.replaced.is_empty() {
        return Ok(plan.report);
    }
    let store = if plan.user {
        super::notes_dir()
    } else {
        plan.project.join(".lbc/knowledge")
    };
    crate::security::permissions::validate_directory(&store)?;
    if !plan.user {
        crate::security::git::ensure_gitignore(&plan.project)?;
    }
    // Existing edits lock the note's parent, including legacy and nested stores.
    let mut lock_paths = vec![super::storage::store_lock_path(&store)];
    for entry in &plan.existing {
        let parent = entry
            .path
            .as_deref()
            .and_then(Path::parent)
            .context("existing knowledge has no source directory")?;
        crate::security::permissions::validate_directory(parent)?;
        lock_paths.push(super::storage::store_lock_path(parent));
    }
    lock_paths.sort();
    lock_paths.dedup();
    let _locks = lock_paths
        .iter()
        .map(|path| crate::security::storage::lock_exclusive(path))
        .collect::<Result<Vec<_>>>()?;
    let current = super::bundle::collect(&plan.project, plan.user)?;
    ensure!(
        snapshot(&current) == snapshot(&plan.existing),
        "knowledge changed since preview; newer content was preserved; retry to review it"
    );
    let mut writes = Vec::new();
    for entry in &plan.incoming {
        if plan.report.duplicates.contains(&entry.id) {
            continue;
        }
        let old = plan.existing.iter().find(|old| old.id == entry.id);
        let target = match old {
            Some(old) => old
                .path
                .clone()
                .context("existing knowledge has no source path")?,
            None => store.join(format!("{}.md", entry.id)),
        };
        crate::security::files::reject_symlinks(&target)?;
        crate::security::permissions::validate_directory(
            target.parent().context("missing target directory")?,
        )?;
        if old.is_some() {
            crate::security::permissions::validate_replacement_target(&target)?;
        } else {
            ensure!(
                fs::symlink_metadata(&target)
                    .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound),
                "destination already exists; nothing was written"
            );
        }
        writes.push((entry, target, old));
    }
    for (entry, target, old) in writes {
        let result = if let Some(old) = old {
            ensure!(
                crate::security::files::read_store_text(&target, 256 * 1024, false)?
                    == old.markdown,
                "knowledge changed before publication; newer content was preserved"
            );
            crate::security::storage::atomic_replace(&target, entry.markdown.as_bytes(), false)
        } else {
            crate::security::storage::atomic_create(&target, entry.markdown.as_bytes(), true)
        };
        result.context("batch stopped; completed notes remain valid; rerun to resume")?;
        after_write()
            .context("batch interrupted; completed notes remain valid; rerun to resume")?;
    }
    Ok(plan.report)
}

fn snapshot(entries: &[Entry]) -> BTreeMap<&str, (&str, Option<&Path>)> {
    entries
        .iter()
        .map(|entry| {
            (
                entry.id.as_str(),
                (entry.markdown.as_str(), entry.path.as_deref()),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn entry(id: &str, body: &str) -> Entry {
        Entry {
            id: id.into(),
            markdown: format!("---\nid: {id}\ncustom_legacy: keep-me\n---\n# Note\n\n{body}\n"),
            path: None,
        }
    }

    #[test]
    fn import_preserves_raw_metadata_and_ids_and_retry_is_idempotent() {
        let root = tempfile::tempdir().unwrap();
        let incoming = entry("stable-id", "Our fix");
        let expected = incoming.markdown.clone();
        let report = apply(
            prepare(root.path(), false, vec![incoming.clone()]).unwrap(),
            false,
        )
        .unwrap();
        assert_eq!(report.added, ["stable-id"]);
        let path = root.path().join(".lbc/knowledge/stable-id.md");
        assert_eq!(fs::read_to_string(&path).unwrap(), expected);
        let again = apply(prepare(root.path(), false, vec![incoming]).unwrap(), false).unwrap();
        assert_eq!(again.duplicates, ["stable-id"]);
        assert!(again.added.is_empty());
    }

    #[test]
    fn conflicts_stop_the_entire_batch_until_replacement_is_explicit() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("knowledge/nested/old.md");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let original = entry("legacy", "Original fix");
        fs::write(&path, &original.markdown).unwrap();
        let incoming = vec![entry("new", "New fix"), entry("legacy", "Replacement fix")];
        let plan = prepare(root.path(), false, incoming.clone()).unwrap();
        assert_eq!(plan.report.conflicts, ["legacy"]);
        assert!(apply(plan, false).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), original.markdown);
        assert!(!root.path().join(".lbc").exists());
        let report = apply(prepare(root.path(), false, incoming).unwrap(), true).unwrap();
        assert_eq!(report.replaced, ["legacy"]);
        assert!(
            fs::read_to_string(&path)
                .unwrap()
                .contains("Replacement fix")
        );
        assert!(!root.path().join(".lbc/knowledge/legacy.md").exists());
    }

    #[test]
    fn newer_content_after_preview_is_preserved() {
        let root = tempfile::tempdir().unwrap();
        let original = entry("one", "Original fix");
        apply(prepare(root.path(), false, vec![original]).unwrap(), false).unwrap();
        let plan = prepare(root.path(), false, vec![entry("one", "Stale replacement")]).unwrap();
        let path = root.path().join(".lbc/knowledge/one.md");
        let newer = entry("one", "Newer concurrent edit");
        fs::write(&path, &newer.markdown).unwrap();
        assert!(apply(plan, true).is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), newer.markdown);
    }

    #[test]
    fn interrupted_batch_leaves_only_complete_notes_and_retry_resumes() {
        let root = tempfile::tempdir().unwrap();
        let incoming = vec![entry("one", "First fix"), entry("two", "Second fix")];
        let plan = prepare(root.path(), false, incoming.clone()).unwrap();
        assert!(apply_with_hook(plan, false, || anyhow::bail!("injected interruption")).is_err());
        assert_eq!(
            fs::read_to_string(root.path().join(".lbc/knowledge/one.md")).unwrap(),
            incoming[0].markdown
        );
        assert!(!root.path().join(".lbc/knowledge/two.md").exists());
        let report = apply(prepare(root.path(), false, incoming).unwrap(), false).unwrap();
        assert_eq!(report.duplicates, ["one"]);
        assert_eq!(report.added, ["two"]);
    }

    #[test]
    fn interrupted_import_child() {
        let Some(project) = std::env::var_os("LBC_TRANSFER_INTERRUPTION_TEST") else {
            return;
        };
        let project = PathBuf::from(project);
        let plan = prepare(
            &project,
            false,
            vec![entry("one", "First fix"), entry("two", "Second fix")],
        )
        .unwrap();
        apply_with_hook(plan, false, || {
            fs::write(project.join("checkpoint"), b"published")?;
            loop {
                std::thread::park();
            }
        })
        .unwrap();
    }

    #[test]
    fn killed_process_leaves_complete_notes_and_releases_locks_for_retry() {
        use std::{
            process::{Command, Stdio},
            time::{Duration, Instant},
        };
        let root = tempfile::tempdir().unwrap();
        let preserved = entry("existing", "Unrelated user content");
        apply(
            prepare(root.path(), false, vec![preserved.clone()]).unwrap(),
            false,
        )
        .unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "knowledge::transfer::tests::interrupted_import_child",
            ])
            .env("LBC_TRANSFER_INTERRUPTION_TEST", root.path())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !root.path().join("checkpoint").exists() && Instant::now() < deadline {
            if child.try_wait().unwrap().is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let checkpoint = root.path().join("checkpoint").exists();
        let _ = child.kill();
        child.wait().unwrap();
        assert!(checkpoint, "child did not reach the publication checkpoint");
        let store = root.path().join(".lbc/knowledge");
        assert_eq!(
            fs::read_to_string(store.join("one.md")).unwrap(),
            entry("one", "First fix").markdown
        );
        assert!(!store.join("two.md").exists());
        assert_eq!(
            fs::read_to_string(store.join("existing.md")).unwrap(),
            preserved.markdown
        );
        let report = apply(
            prepare(
                root.path(),
                false,
                vec![entry("one", "First fix"), entry("two", "Second fix")],
            )
            .unwrap(),
            false,
        )
        .unwrap();
        assert_eq!(report.duplicates, ["one"]);
        assert_eq!(report.added, ["two"]);
    }
}
