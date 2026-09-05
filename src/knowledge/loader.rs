use std::{
    collections::HashMap,
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
    if let Ok(entries) = fs::read_dir(package_dir) {
        let mut entries: Vec<_> = entries
            .flatten()
            .filter(|entry| entry.path().is_dir())
            .collect();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let manifest_path = entry.path().join("package.toml");
            let manifest = fs::read_to_string(&manifest_path)
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
    mark_duplicate_ids(&mut documents, &mut invalid);
    apply_overrides(&mut documents, &mut invalid);
    documents.sort_by(|left, right| {
        left.source_id
            .cmp(&right.source_id)
            .then(left.path.cmp(&right.path))
    });
    Ok(LoadReport { documents, invalid })
}

fn collect_source(
    root: &Path,
    source: &str,
    locator_root: &Path,
    writable: bool,
    documents: &mut Vec<KnowledgeDocument>,
    invalid: &mut Vec<InvalidDocument>,
) {
    if !root.is_dir() {
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
        if !entry.file_type().is_file()
            || entry
                .path()
                .extension()
                .and_then(|extension| extension.to_str())
                != Some("md")
        {
            continue;
        }
        let path = entry.path();
        let display = path.display().to_string();
        let parsed = (|| -> Result<KnowledgeDocument> {
            if entry.metadata()?.len() > MAX_DOCUMENT_BYTES {
                anyhow::bail!("document is larger than 256 KB");
            }
            let content = fs::read_to_string(path)?;
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
        if priority == 0 || !target.contains(':') {
            document.effective = false;
            invalid.push(InvalidDocument { path: document.path.clone(), error: format!("invalid override target {target:?}; only user/project notes may override a source-qualified ID") });
            continue;
        }
        groups.entry(target).or_default().push((priority, position));
    }
    for (target, candidates) in groups {
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
    let content = content.replace('\r', "");
    let Some(after_opening) = content.strip_prefix("---\n") else {
        anyhow::bail!("missing YAML frontmatter");
    };
    let Some((frontmatter, body)) = after_opening.split_once("\n---\n") else {
        anyhow::bail!("unterminated YAML frontmatter");
    };
    let metadata: KnowledgeMetadata = serde_yaml::from_str(frontmatter)?;
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
}
