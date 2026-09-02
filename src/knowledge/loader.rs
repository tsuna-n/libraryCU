use std::{collections::HashSet, fs, path::Path};

use anyhow::{Context, Result};
use walkdir::WalkDir;

use super::document::{KnowledgeDocument, KnowledgeMetadata};
use super::packages;

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

/// Load built-in knowledge, installed knowledge packages, then the project's
/// own documents. Built-in IDs always win; later sources only extend the store.
pub fn load_documents(project_root: &Path) -> Result<Vec<KnowledgeDocument>> {
    load_documents_with_data_dir(project_root, &packages::data_dir())
}

pub fn load_documents_with_data_dir(
    project_root: &Path,
    user_data_dir: &Path,
) -> Result<Vec<KnowledgeDocument>> {
    let mut documents = Vec::new();
    let mut ids = HashSet::new();
    for (path, content) in BUILTIN_DOCUMENTS {
        let document = parse_document(path, content)
            .with_context(|| format!("invalid built-in knowledge document {path}"))?;
        ids.insert(document.metadata.id.clone());
        documents.push(document);
    }

    let user_root = user_data_dir.to_path_buf();
    for (relative, content) in collect_markdown(&user_root) {
        if let Ok(document) = parse_document(&relative, &content)
            && ids.insert(document.metadata.id.clone())
        {
            documents.push(document);
        }
    }

    let knowledge_root = project_root.join("knowledge");
    for (relative, content) in collect_markdown(&knowledge_root) {
        if let Ok(document) = parse_document(&relative, &content)
            && ids.insert(document.metadata.id.clone())
        {
            documents.push(document);
        }
    }
    Ok(documents)
}

/// Collect every readable markdown file below `root` as (relative path, content).
fn collect_markdown(root: &Path) -> Vec<(String, String)> {
    let mut files = Vec::new();
    if !root.is_dir() {
        return files;
    }
    for entry in WalkDir::new(root).follow_links(false) {
        let Ok(entry) = entry else {
            continue;
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
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if metadata.len() > 256 * 1024 {
            continue;
        }
        let Ok(content) = fs::read_to_string(entry.path()) else {
            continue;
        };
        let relative = entry
            .path()
            .strip_prefix(root)
            .unwrap_or(entry.path())
            .to_string_lossy()
            .into_owned();
        files.push((relative, content));
    }
    files.sort();
    files
}

pub fn parse_document(path: &str, content: &str) -> Result<KnowledgeDocument> {
    let content = content.replace('\r', "");
    let Some(after_opening) = content.strip_prefix("---\n") else {
        anyhow::bail!("missing YAML frontmatter");
    };
    let Some((frontmatter, body)) = after_opening.split_once("\n---\n") else {
        anyhow::bail!("unterminated YAML frontmatter");
    };
    let metadata: KnowledgeMetadata = serde_yaml::from_str(frontmatter)?;
    if metadata.id.trim().is_empty() {
        anyhow::bail!("knowledge id must not be empty");
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
    Ok(KnowledgeDocument {
        metadata,
        title,
        body: body.trim().to_owned(),
        path: path.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        std::env::temp_dir().join(format!("lbc-load-{name}-{}-{nonce}", std::process::id()))
    }

    #[test]
    fn loads_builtin_knowledge() -> Result<()> {
        let documents = load_documents_with_data_dir(Path::new("/missing"), Path::new("/missing"))?;
        assert!(documents.iter().any(|document| {
            document.metadata.error_code.as_deref() == Some("E0382")
                && document.title.contains("moved value")
        }));
        Ok(())
    }

    #[test]
    fn loads_all_four_knowledge_domains() -> Result<()> {
        let documents = load_documents_with_data_dir(Path::new("/missing"), Path::new("/missing"))?;
        for tool in ["rust", "linux", "git", "docker"] {
            assert!(
                documents
                    .iter()
                    .any(
                        |document| document.metadata.language.as_deref() == Some(tool)
                            || document.metadata.tool.as_deref() == Some(tool)
                    ),
                "no documents found for {tool}"
            );
        }
        Ok(())
    }

    #[test]
    fn user_packages_extend_but_not_replace_builtin_ids() -> Result<()> {
        let data = temp_dir("packages");
        let package = data.join("demo-pack");
        fs::create_dir_all(&package)?;
        fs::write(
            package.join("package.toml"),
            "name = \"demo-pack\"\nversion = \"0.1.0\"\n",
        )?;
        fs::write(
            package.join("extra.md"),
            "---\nid: rust-e0382\ntitle: Fake Override\n---\n# Fake Override\n\nBody.\n",
        )?;
        fs::write(
            package.join("fresh.md"),
            "---\nid: demo-fresh\ntitle: Fresh Doc\n---\n# Fresh Doc\n\nBody.\n",
        )?;

        let documents = load_documents_with_data_dir(Path::new("/missing"), &data)?;
        let e0382 = documents
            .iter()
            .find(|document| document.metadata.id == "rust-e0382")
            .expect("built-in E0382 should exist");
        assert_ne!(e0382.title, "Fake Override");
        assert!(
            documents
                .iter()
                .any(|document| document.metadata.id == "demo-fresh")
        );
        fs::remove_dir_all(data)?;
        Ok(())
    }

    #[test]
    fn frontmatter_title_wins_over_heading() -> Result<()> {
        let document = parse_document(
            "demo.md",
            "---\nid: demo\ntitle: From Metadata\n---\n# From Heading\n\nBody.\n",
        )?;
        assert_eq!(document.title, "From Metadata");
        Ok(())
    }
}
