use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::Serialize;
use walkdir::WalkDir;

use super::document::{KnowledgeDocument, KnowledgeMetadata};
use super::packages;

const MAX_DOCUMENT_BYTES: u64 = 256 * 1024;
const BUILTIN_DOCUMENTS: &[(&str, &str)] = &[
    (
        "rust/E0382.md",
        include_str!("../../knowledge/rust/E0382.md"),
    ),
    (
        "rust/E0308.md",
        include_str!("../../knowledge/rust/E0308.md"),
    ),
    (
        "rust/E0432.md",
        include_str!("../../knowledge/rust/E0432.md"),
    ),
    (
        "rust/E0433.md",
        include_str!("../../knowledge/rust/E0433.md"),
    ),
    (
        "rust/E0499.md",
        include_str!("../../knowledge/rust/E0499.md"),
    ),
    (
        "rust/ownership.md",
        include_str!("../../knowledge/rust/ownership.md"),
    ),
    (
        "linux/permission-denied.md",
        include_str!("../../knowledge/linux/permission-denied.md"),
    ),
    (
        "linux/command-not-found.md",
        include_str!("../../knowledge/linux/command-not-found.md"),
    ),
    (
        "git/merge-conflict.md",
        include_str!("../../knowledge/git/merge-conflict.md"),
    ),
    (
        "git/detached-head.md",
        include_str!("../../knowledge/git/detached-head.md"),
    ),
    (
        "docker/port-in-use.md",
        include_str!("../../knowledge/docker/port-in-use.md"),
    ),
    (
        "docker/daemon-not-running.md",
        include_str!("../../knowledge/docker/daemon-not-running.md"),
    ),
];

#[derive(Debug, Clone, Serialize)]
pub struct InvalidDocument {
    pub path: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoadReport {
    pub documents: Vec<KnowledgeDocument>,
    pub invalid: Vec<InvalidDocument>,
}

pub fn load_documents(project_root: &Path) -> Result<Vec<KnowledgeDocument>> {
    Ok(load_all_documents(project_root)?
        .documents
        .into_iter()
        .filter(|document| document.effective)
        .collect())
}

pub fn load_all_documents(project_root: &Path) -> Result<LoadReport> {
    load_all_documents_with_roots(
        project_root,
        &packages::data_dir(),
        &super::storage::notes_dir(),
    )
}

/// Compatibility helper retained for package and unit tests.
pub fn load_documents_with_data_dir(
    project_root: &Path,
    package_dir: &Path,
) -> Result<Vec<KnowledgeDocument>> {
    let notes = package_dir
        .parent()
        .map(|parent| parent.join("notes"))
        .unwrap_or_else(|| PathBuf::from("notes"));
    Ok(
        load_all_documents_with_roots(project_root, package_dir, &notes)?
            .documents
            .into_iter()
            .filter(|document| document.effective)
            .collect(),
    )
}

fn load_all_documents_with_roots(
    project_root: &Path,
    package_dir: &Path,
    notes_dir: &Path,
) -> Result<LoadReport> {
    let mut documents = Vec::new();
    let mut invalid = Vec::new();
    for (path, content) in BUILTIN_DOCUMENTS {
        documents.push(
            parse_document_for_source(path, content, "builtin", path, false)
                .with_context(|| format!("invalid built-in knowledge document {path}"))?,
        );
    }
    collect_source(
        notes_dir,
        "user",
        notes_dir,
        true,
        &mut documents,
        &mut invalid,
    );
    let package_root_valid = crate::security::files::reject_symlinks(package_dir);
    if let Err(error) = &package_root_valid {
        invalid.push(InvalidDocument {
            path: package_dir.display().to_string(),
            error: error.to_string(),
        });
    }
    let entries = package_root_valid.ok().map(|()| fs::read_dir(package_dir));
    if let Some(Err(error)) = &entries
        && package_dir.exists()
    {
        invalid.push(InvalidDocument {
            path: package_dir.display().to_string(),
            error: format!("failed to inspect package directory: {error}"),
        });
    }
    if let Some(Ok(entries)) = entries {
        let mut directories = Vec::new();
        for entry in entries {
            match entry {
                Ok(entry)
                    if !entry.file_name().to_string_lossy().starts_with('.')
                        && entry.path().is_dir() =>
                {
                    directories.push(entry)
                }
                Ok(_) => {}
                Err(error) => invalid.push(InvalidDocument {
                    path: package_dir.display().to_string(),
                    error: format!("failed to inspect package entry: {error}"),
                }),
            }
        }
        let mut entries = directories;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            if packages::package_has_integrity_manifest(&entry.path())
                && let Err(error) = packages::verify_package_integrity(&entry.path())
            {
                invalid.push(InvalidDocument {
                    path: entry.path().display().to_string(),
                    error: format!("package integrity verification failed: {error}"),
                });
                continue;
            }
            let manifest_path = entry.path().join("package.toml");
            let manifest = crate::security::files::read_text(&manifest_path, MAX_DOCUMENT_BYTES)
                .with_context(|| format!("failed to read {}", manifest_path.display()))
                .and_then(|content| packages::parse_manifest(&content));
            let manifest = match manifest {
                Ok(manifest) => manifest,
                Err(error) => {
                    invalid.push(InvalidDocument {
                        path: manifest_path.display().to_string(),
                        error: error.to_string(),
                    });
                    continue;
                }
            };
            if entry.file_name() != std::ffi::OsStr::new(&manifest.name) {
                invalid.push(InvalidDocument {
                    path: manifest_path.display().to_string(),
                    error: "package manifest name does not match its directory name".to_owned(),
                });
                continue;
            }
            let source = format!("package:{}", manifest.name);
            collect_source(
                &entry.path(),
                &source,
                &entry.path(),
                false,
                &mut documents,
                &mut invalid,
            );
        }
    }
    let legacy = project_root.join("knowledge");
    collect_source(
        &legacy,
        "project",
        &legacy,
        true,
        &mut documents,
        &mut invalid,
    );
    let explicit = project_root.join(".lbc/knowledge");
    collect_source(
        &explicit,
        "project",
        &explicit,
        true,
        &mut documents,
        &mut invalid,
    );
    mark_repeated_content(&mut documents);
    mark_duplicate_ids(&mut documents, &mut invalid);
    apply_overrides(&mut documents, &mut invalid);
    documents.sort_by(|left, right| {
        left.source_id
            .cmp(&right.source_id)
            .then(left.path.cmp(&right.path))
    });
    Ok(LoadReport { documents, invalid })
}

/// Keep one effective copy when the same note is both embedded in the binary
/// and visible from a source checkout (or copied unchanged into another store).
fn mark_repeated_content(documents: &mut [KnowledgeDocument]) {
    let mut seen: HashMap<(KnowledgeMetadata, String), usize> = HashMap::new();
    for position in 0..documents.len() {
        let key = (
            documents[position].metadata.clone(),
            documents[position].body.clone(),
        );
        if let Some(previous) = seen.get(&key).copied() {
            let previous_priority = source_priority(&documents[previous].source);
            let current_priority = source_priority(&documents[position].source);
            if current_priority > previous_priority {
                documents[previous].effective = false;
                documents[previous].overridden_by = Some(documents[position].source_id.clone());
                seen.insert(key, position);
            } else {
                documents[position].effective = false;
                documents[position].overridden_by = Some(documents[previous].source_id.clone());
            }
        } else {
            seen.insert(key, position);
        }
    }
}

fn source_priority(source: &str) -> u8 {
    match source {
        // Stable built-in identities are preferable when another store holds
        // a byte-for-byte copy of an embedded note.
        "builtin" => 4,
        "project" => 3,
        "user" => 2,
        _ => 1,
    }
}

fn collect_source(
    root: &Path,
    source: &str,
    locator_root: &Path,
    writable: bool,
    documents: &mut Vec<KnowledgeDocument>,
    invalid: &mut Vec<InvalidDocument>,
) {
    if let Err(error) = crate::security::files::reject_symlinks(root) {
        invalid.push(InvalidDocument {
            path: root.display().to_string(),
            error: error.to_string(),
        });
        return;
    }
    if !root.is_dir() {
        if root.exists() {
            invalid.push(InvalidDocument {
                path: root.display().to_string(),
                error: "knowledge source must be a directory".to_owned(),
            });
        }
        return;
    }
    for result in WalkDir::new(root).follow_links(false).sort_by_file_name() {
        let entry = match result {
            Ok(entry) => entry,
            Err(error) => {
                invalid.push(InvalidDocument {
                    path: root.display().to_string(),
                    error: format!("failed to inspect knowledge path: {error}"),
                });
                continue;
            }
        };
        if entry.file_type().is_dir()
            || entry
                .path()
                .extension()
                .and_then(|extension| extension.to_str())
                != Some("md")
        {
            continue;
        }
        if !entry.file_type().is_file() {
            invalid.push(InvalidDocument {
                path: entry.path().display().to_string(),
                error: "knowledge document must be a regular file, without symlinks".to_owned(),
            });
            continue;
        }
        let path = entry.path();
        let display = path.display().to_string();
        let parsed = (|| -> Result<KnowledgeDocument> {
            if entry.metadata()?.len() > MAX_DOCUMENT_BYTES {
                anyhow::bail!("document is larger than 256 KB");
            }
            let content = crate::security::files::read_text(path, MAX_DOCUMENT_BYTES)?;
            let relative = path
                .strip_prefix(locator_root)
                .unwrap_or(path)
                .to_string_lossy();
            parse_document_for_source(&relative, &content, source, &display, writable)
        })();
        match parsed {
            Ok(document) => documents.push(document),
            Err(error) => invalid.push(InvalidDocument {
                path: display,
                error: error.to_string(),
            }),
        }
    }
}

fn mark_duplicate_ids(documents: &mut [KnowledgeDocument], invalid: &mut Vec<InvalidDocument>) {
    let mut seen: HashMap<(String, String), usize> = HashMap::new();
    for position in 0..documents.len() {
        let key = (
            documents[position].source.clone(),
            documents[position].metadata.id.clone(),
        );
        if let Some(first) = seen.get(&key).copied() {
            documents[position].effective = false;
            documents[first].effective = false;
            invalid.push(InvalidDocument {
                path: documents[position].path.clone(),
                error: format!(
                    "duplicate ID {} in source {}; first seen at {}",
                    key.1, key.0, documents[first].path
                ),
            });
        } else {
            seen.insert(key, position);
        }
    }
}

fn apply_overrides(documents: &mut [KnowledgeDocument], invalid: &mut Vec<InvalidDocument>) {
    let positions: HashMap<_, _> = documents
        .iter()
        .enumerate()
        .map(|(position, document)| (document.source_id.clone(), position))
        .collect();
    let mut cycle_members = HashSet::new();
    for start in 0..documents.len() {
        if documents[start].metadata.overrides.is_none() || !documents[start].effective {
            continue;
        }
        let mut path = Vec::new();
        let mut current = start;
        loop {
            if let Some(cycle_start) = path.iter().position(|position| *position == current) {
                cycle_members.extend(path[cycle_start..].iter().copied());
                break;
            }
            path.push(current);
            let Some(target) = documents[current].metadata.overrides.as_ref() else {
                break;
            };
            let Some(next) = positions.get(target).copied() else {
                break;
            };
            current = next;
        }
    }
    for position in cycle_members {
        documents[position].effective = false;
        let target = documents[position]
            .metadata
            .overrides
            .as_deref()
            .unwrap_or_default();
        let error = if target == documents[position].source_id {
            "knowledge entry cannot override itself".to_owned()
        } else {
            format!("override cycle includes {}", documents[position].source_id)
        };
        invalid.push(InvalidDocument {
            path: documents[position].path.clone(),
            error,
        });
    }

    let mut groups: HashMap<String, Vec<(u8, usize)>> = HashMap::new();
    for (position, document) in documents.iter_mut().enumerate() {
        let Some(target) = document.metadata.overrides.clone() else {
            continue;
        };
        if !document.effective {
            continue;
        }
        let priority = match document.source.as_str() {
            "project" => 2,
            "user" => 1,
            _ => 0,
        };
        if priority == 0 || !valid_reference(&target, true) {
            document.effective = false;
            invalid.push(InvalidDocument { path: document.path.clone(), error: format!("invalid override target {target:?}; only user/project notes may override a source-qualified ID") });
            continue;
        }
        groups.entry(target).or_default().push((priority, position));
    }
    for (target, candidates) in groups {
        if documents
            .iter()
            .filter(|document| document.source_id == target)
            .count()
            > 1
        {
            for (_, position) in candidates {
                documents[position].effective = false;
                invalid.push(InvalidDocument {
                    path: documents[position].path.clone(),
                    error: format!("override target {target} is ambiguous"),
                });
            }
            continue;
        }
        let Some(target_position) = documents
            .iter()
            .position(|document| document.source_id == target)
        else {
            for (_, position) in candidates {
                documents[position].effective = false;
                invalid.push(InvalidDocument {
                    path: documents[position].path.clone(),
                    error: format!("override target {target} does not exist"),
                });
            }
            continue;
        };
        let highest = candidates
            .iter()
            .map(|(priority, _)| *priority)
            .max()
            .unwrap_or(0);
        let highest_positions: Vec<_> = candidates
            .iter()
            .filter(|(priority, _)| *priority == highest)
            .map(|(_, position)| *position)
            .collect();
        if highest_positions.len() != 1 {
            for position in highest_positions {
                documents[position].effective = false;
                invalid.push(InvalidDocument {
                    path: documents[position].path.clone(),
                    error: format!("multiple equally-ranked overrides target {target}"),
                });
            }
            continue;
        }
        let winner = highest_positions[0];
        let winner_id = documents[winner].source_id.clone();
        documents[target_position].effective = false;
        documents[target_position].overridden_by = Some(winner_id.clone());
        for (_, position) in candidates {
            if position != winner {
                documents[position].effective = false;
                documents[position].overridden_by = Some(winner_id.clone());
            }
        }
    }
}

pub fn parse_document(path: &str, content: &str) -> Result<KnowledgeDocument> {
    parse_document_for_source(path, content, "unknown", path, false)
}

pub fn parse_document_for_source(
    relative_path: &str,
    content: &str,
    source: &str,
    locator: &str,
    writable: bool,
) -> Result<KnowledgeDocument> {
    if content.len() as u64 > MAX_DOCUMENT_BYTES {
        anyhow::bail!("knowledge document including metadata exceeds 256 KB");
    }
    if content.contains('\0') {
        anyhow::bail!("knowledge document must not contain NUL characters");
    }
    let content = content.replace('\r', "");
    let Some(after_opening) = content.strip_prefix("---\n") else {
        anyhow::bail!("missing YAML frontmatter");
    };
    let Some((frontmatter, body)) = after_opening.split_once("\n---\n") else {
        anyhow::bail!("unterminated YAML frontmatter");
    };
    let metadata: KnowledgeMetadata = serde_yaml::from_str(frontmatter).map_err(|error| {
        // Parser messages can echo arbitrary private values from the document.
        let location = error.location().map_or(String::new(), |location| {
            format!(
                " at line {} column {}",
                location.line() + 1,
                location.column()
            )
        });
        anyhow::anyhow!("invalid YAML frontmatter{location}; check field names and value types")
    })?;
    validate_id(&metadata.id)?;
    if body.trim().is_empty() {
        anyhow::bail!("knowledge body must not be empty");
    }
    if let Some(kind) = metadata.kind.as_deref()
        && !matches!(kind, "note" | "concept" | "troubleshooting")
    {
        anyhow::bail!("kind must be note, concept, or troubleshooting");
    }
    if let Some(status) = metadata.verification_status.as_deref()
        && !matches!(status, "unverified" | "user-reported" | "recorded-check")
    {
        anyhow::bail!("verification_status must be unverified, user-reported, or recorded-check");
    }
    let title = metadata
        .title
        .clone()
        .filter(|title| !title.trim().is_empty())
        .or_else(|| {
            body.lines()
                .find_map(|line| line.trim().strip_prefix("# "))
                .map(str::to_owned)
        })
        .unwrap_or_else(|| metadata.id.clone());
    if title.trim().is_empty() {
        anyhow::bail!("knowledge title must not be empty");
    }
    let source_id = format!("{source}:{}", metadata.id);
    let kind = metadata.kind.clone().unwrap_or_else(|| {
        if metadata.error_code.is_some() {
            "troubleshooting".to_owned()
        } else {
            "note".to_owned()
        }
    });
    let verification_status = metadata
        .verification_status
        .clone()
        .unwrap_or_else(|| "unverified".to_owned());
    Ok(KnowledgeDocument {
        metadata,
        title,
        body: body.trim().to_owned(),
        path: if source == "builtin" {
            format!("embedded:{relative_path}")
        } else {
            locator.to_owned()
        },
        source: source.to_owned(),
        source_id,
        kind,
        verification_status,
        writable,
        effective: true,
        overridden_by: None,
    })
}

pub fn validate_id(id: &str) -> Result<()> {
    if id.is_empty()
        || id.len() > 96
        || !id.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
    {
        anyhow::bail!(
            "knowledge id must contain only letters, digits, '-', '_' or '.' (maximum 96 characters)"
        );
    }
    Ok(())
}

pub(crate) fn valid_reference(reference: &str, require_qualified: bool) -> bool {
    let Some((source, id)) = reference.rsplit_once(':') else {
        return !require_qualified && validate_id(reference).is_ok();
    };
    validate_id(id).is_ok()
        && (matches!(source, "builtin" | "user" | "project")
            || source
                .strip_prefix("package:")
                .is_some_and(packages::is_valid_package_name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_root(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        std::env::temp_dir().join(format!("lbc-loader-{name}-{}-{nonce}", std::process::id()))
    }
    #[test]
    fn loads_builtin_knowledge_with_qualified_identity() -> Result<()> {
        let documents = load_documents_with_data_dir(Path::new("/missing"), Path::new("/missing"))?;
        let item = documents
            .iter()
            .find(|document| document.metadata.error_code.as_deref() == Some("E0308"))
            .expect("E0308");
        assert!(item.source_id.starts_with("builtin:"));
        assert!(item.path.starts_with("embedded:"));
        Ok(())
    }
    #[test]
    fn validates_body_and_kind() {
        assert!(parse_document("bad.md", "---\nid: x\n---\n").is_err());
        assert!(parse_document("bad.md", "---\nid: x\nkind: other\n---\nbody").is_err());
    }

    #[test]
    fn rejects_nul_in_markdown_body() {
        assert!(parse_document("bad.md", "---\nid: x\n---\nbody\0hidden").is_err());
    }

    #[test]
    fn ignores_package_staging_and_rejects_package_aliases() -> Result<()> {
        let root = tempfile::tempdir()?;
        let packages = root.path().join("packages");
        fs::create_dir_all(packages.join(".staging"))?;
        fs::create_dir_all(packages.join("alias"))?;
        fs::write(
            packages.join("alias/package.toml"),
            "name = 'actual'\nversion = '1'\ndescription = 'Alias'\n",
        )?;
        fs::write(packages.join("alias/note.md"), "---\nid: alias\n---\nbody")?;

        let report = load_all_documents_with_roots(
            &root.path().join("project"),
            &packages,
            &root.path().join("notes"),
        )?;

        assert_eq!(report.invalid.len(), 1);
        assert!(report.invalid[0].error.contains("directory name"));
        assert!(
            report
                .documents
                .iter()
                .all(|document| document.source != "package:actual")
        );
        Ok(())
    }

    #[test]
    fn reports_bad_metadata_without_creating_stores_or_changing_notes() -> Result<()> {
        let root = tempfile::tempdir()?;
        let project = root.path().join("project");
        let notes = project.join(".lbc/knowledge");
        fs::create_dir_all(&notes)?;
        let fixtures = [
            ("frontmatter.md", "# No frontmatter"),
            (
                "references.md",
                "---\nid: refs\nreferences: {invalid: shape}\n---\nbody",
            ),
            ("date.md", "---\nid: date\ncreated_at_unix: -1\n---\nbody"),
        ];
        for (name, content) in fixtures {
            fs::write(notes.join(name), content)?;
        }
        let packages = root.path().join("packages");
        let user = root.path().join("user");

        let report = load_all_documents_with_roots(&project, &packages, &user)?;
        let validation = super::super::validation::validate_report(&report, 100, 365);

        assert_eq!(report.invalid.len(), 3);
        assert_eq!(
            validation
                .warnings
                .iter()
                .filter(|warning| warning.code == "invalid_document")
                .count(),
            3
        );
        for (name, content) in fixtures {
            assert_eq!(fs::read_to_string(notes.join(name))?, content);
        }
        assert!(!packages.exists());
        assert!(!user.exists());
        assert!(!project.join(".lbc/index.db").exists());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn reports_symlinked_notes_without_reading_them() -> Result<()> {
        let root = tempfile::tempdir()?;
        let notes = root.path().join("notes");
        fs::create_dir(&notes)?;
        let target = root.path().join("private.txt");
        fs::write(&target, "private body")?;
        std::os::unix::fs::symlink(&target, notes.join("linked.md"))?;

        let report = load_all_documents_with_roots(
            &root.path().join("project"),
            &root.path().join("packages"),
            &notes,
        )?;
        assert_eq!(report.invalid.len(), 1);
        assert!(report.invalid[0].error.contains("regular file"));
        assert!(!report.invalid[0].error.contains("private body"));
        Ok(())
    }

    #[test]
    fn rejects_overrides_of_duplicate_qualified_ids() -> Result<()> {
        let mut documents = vec![
            parse_document_for_source("a.md", "---\nid: same\n---\nA", "user", "/a.md", true)?,
            parse_document_for_source("b.md", "---\nid: same\n---\nB", "user", "/b.md", true)?,
            parse_document_for_source(
                "c.md",
                "---\nid: replacement\noverrides: user:same\n---\nC",
                "project",
                "/c.md",
                true,
            )?,
        ];
        let mut invalid = Vec::new();
        mark_duplicate_ids(&mut documents, &mut invalid);
        apply_overrides(&mut documents, &mut invalid);
        assert!(!documents[2].effective);
        assert!(invalid.iter().any(|item| item.error.contains("ambiguous")));
        Ok(())
    }

    #[test]
    fn yaml_errors_keep_location_without_echoing_private_values() {
        let error = parse_document(
            "private.md",
            "---\nid: private\ntags: unlabelled-private-marker\n---\nbody",
        )
        .unwrap_err();
        let message = error.to_string();
        assert!(!message.contains("unlabelled-private-marker"));
        assert!(message.contains("line 3 column"), "{message}");
        assert!(message.contains("check field names and value types"));
    }

    #[test]
    fn repeated_content_keeps_the_stable_builtin_identity() -> Result<()> {
        let content = "---\nid: same\ntitle: Same note\n---\nSame body";
        let mut documents = vec![
            parse_document_for_source("same.md", content, "builtin", "embedded:same.md", false)?,
            parse_document_for_source("same.md", content, "project", "/project/same.md", true)?,
        ];

        mark_repeated_content(&mut documents);

        assert!(documents[0].effective);
        assert!(!documents[1].effective);
        assert_eq!(documents[1].overridden_by.as_deref(), Some("builtin:same"));
        Ok(())
    }

    #[test]
    fn project_override_takes_precedence_over_user_override() -> Result<()> {
        let root = temporary_root("override-priority");
        let notes = root.join("notes");
        let project_notes = root.join("project/.lbc/knowledge");
        fs::create_dir_all(&notes)?;
        fs::create_dir_all(&project_notes)?;
        fs::write(
            notes.join("user.md"),
            "---\nid: user-e0308\noverrides: builtin:rust-e0308\n---\nUSER OVERRIDE\n",
        )?;
        fs::write(
            project_notes.join("project.md"),
            "---\nid: project-e0308\noverrides: builtin:rust-e0308\n---\nPROJECT OVERRIDE\n",
        )?;
        let report =
            load_all_documents_with_roots(&root.join("project"), &root.join("packages"), &notes)?;
        assert!(report.invalid.is_empty());
        assert!(report.documents.iter().any(|document| {
            document.source_id == "project:project-e0308" && document.effective
        }));
        assert!(report.documents.iter().any(|document| {
            document.source_id == "user:user-e0308"
                && !document.effective
                && document.overridden_by.as_deref() == Some("project:project-e0308")
        }));
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn rejects_self_overrides_and_override_cycles() -> Result<()> {
        let self_override =
            "---\nid: self\noverrides: user:self\n---\nSELF OVERRIDE MUST NOT APPLY\n";
        let first = "---\nid: first\noverrides: user:second\n---\nFIRST\n";
        let second = "---\nid: second\noverrides: user:first\n---\nSECOND\n";
        let mut documents = vec![
            parse_document_for_source("self.md", self_override, "user", "/self.md", true)?,
            parse_document_for_source("first.md", first, "user", "/first.md", true)?,
            parse_document_for_source("second.md", second, "user", "/second.md", true)?,
        ];
        let mut invalid = Vec::new();

        apply_overrides(&mut documents, &mut invalid);

        assert!(documents.iter().all(|document| !document.effective));
        assert!(
            invalid
                .iter()
                .any(|item| item.error.contains("cannot override itself"))
        );
        assert_eq!(
            invalid
                .iter()
                .filter(|item| item.error.contains("override cycle"))
                .count(),
            2
        );
        Ok(())
    }
}
