use super::document::KnowledgeDocument;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub document: KnowledgeDocument,
    pub score: u32,
    pub match_reason: String,
}

pub fn search(documents: &[KnowledgeDocument], query: &str) -> Vec<SearchResult> {
    let normalized = query.trim().to_lowercase();
    if normalized.is_empty() {
        return Vec::new();
    }
    let terms: Vec<_> = normalized
        .split(|character: char| !character.is_alphanumeric())
        .filter(|term| !term.is_empty())
        .collect();
    let mut results = Vec::new();

    for document in documents {
        let title = document.title.to_lowercase();
        let body = document.body.to_lowercase();
        let tags = document.metadata.tags.join(" ").to_lowercase();
        let error_code = document
            .metadata
            .error_code
            .as_deref()
            .unwrap_or_default()
            .to_lowercase();
        let mut score = 0;
        let mut reason = "keyword match";

        if error_code == normalized {
            score += 1_000;
            reason = "exact error code";
        }
        if title.contains(&normalized) {
            score += 100;
            if reason == "keyword match" {
                reason = "title match";
            }
        }
        if tags.contains(&normalized) {
            score += 80;
            if reason == "keyword match" {
                reason = "metadata match";
            }
        }
        for term in &terms {
            if title.contains(term) {
                score += 20;
            }
            if tags.contains(term) {
                score += 12;
            }
            if body.contains(term) {
                score += 3;
            }
        }
        if score > 0 {
            results.push(SearchResult {
                document: document.clone(),
                score,
                match_reason: reason.to_owned(),
            });
        }
    }
    results.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.document.title.cmp(&right.document.title))
    });
    results
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use anyhow::{Context, Result};

    use crate::knowledge::loader::load_documents;

    use super::*;

    #[test]
    fn exact_error_code_is_ranked_first() -> Result<()> {
        let documents = load_documents(Path::new("/missing"))?;
        let results = search(&documents, "E0382");
        let first = results.first().context("expected a search result")?;
        assert_eq!(first.document.metadata.error_code.as_deref(), Some("E0382"));
        assert_eq!(first.match_reason, "exact error code");
        Ok(())
    }

    #[test]
    fn searches_titles_tags_and_content() -> Result<()> {
        let documents = load_documents(Path::new("/missing"))?;
        let results = search(&documents, "borrow checker");
        assert!(!results.is_empty());
        assert!(
            results
                .iter()
                .any(|result| result.document.title.contains("Borrow"))
        );
        Ok(())
    }
}
