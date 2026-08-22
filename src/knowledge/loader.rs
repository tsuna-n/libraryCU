use std::{collections::HashSet, fs, path::Path};

use anyhow::{Context, Result, bail};
use walkdir::WalkDir;

use super::document::{KnowledgeDocument, KnowledgeMetadata};

const BUILTIN_DOCUMENTS: &[(&str, &str)] = &[
    (
        "rust/E0382.md",
        include_str!("../../knowledge/rust/E0382.md"),
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
];

pub fn load_documents(project_root: &Path) -> Result<Vec<KnowledgeDocument>> {
    let mut documents = Vec::new();
    let mut ids = HashSet::new();
    for (path, content) in BUILTIN_DOCUMENTS {
        let document = parse_document(path, content)
            .with_context(|| format!("invalid built-in knowledge document {path}"))?;
        ids.insert(document.metadata.id.clone());
        documents.push(document);
    }

    let knowledge_root = project_root.join("knowledge");
    if !knowledge_root.is_dir() {
        return Ok(documents);
    }
    for result in WalkDir::new(&knowledge_root).follow_links(false) {
        let Ok(entry) = result else {
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
        let path = entry
            .path()
            .strip_prefix(&knowledge_root)
            .unwrap_or(entry.path())
            .to_string_lossy();
        let Ok(document) = parse_document(&path, &content) else {
            continue;
        };
        if ids.insert(document.metadata.id.clone()) {
            documents.push(document);
        }
    }
    Ok(documents)
}

fn parse_document(path: &str, content: &str) -> Result<KnowledgeDocument> {
    let content = content.replace('\r', "");
    let Some(after_opening) = content.strip_prefix("---\n") else {
        bail!("missing YAML frontmatter");
    };
    let Some((frontmatter, body)) = after_opening.split_once("\n---\n") else {
        bail!("unterminated YAML frontmatter");
    };
    let metadata: KnowledgeMetadata = serde_yaml::from_str(frontmatter)?;
    if metadata.id.trim().is_empty() {
        bail!("knowledge id must not be empty");
    }
    let title = body
        .lines()
        .find_map(|line| line.trim().strip_prefix("# "))
        .unwrap_or(metadata.id.as_str())
        .to_owned();
    Ok(KnowledgeDocument {
        metadata,
        title,
        body: body.trim().to_owned(),
        path: path.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_builtin_knowledge() -> Result<()> {
        let documents = load_documents(Path::new("/path/that/does/not/exist"))?;
        assert!(documents.iter().any(|document| {
            document.metadata.error_code.as_deref() == Some("E0382")
                && document.title.contains("moved value")
        }));
        Ok(())
    }
}
