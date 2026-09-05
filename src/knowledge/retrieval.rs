use std::path::Path;

use anyhow::Result;

use super::{InvalidDocument, KnowledgeIndex, SearchResult, load_all_documents};

pub const MAX_QUERY_BYTES: usize = 8 * 1024;

pub fn validate_query(query: &str) -> Result<()> {
    if query.trim().is_empty() {
        anyhow::bail!("question/search query is empty; provide some text");
    }
    if query.len() > MAX_QUERY_BYTES {
        anyhow::bail!(
            "question/search query exceeds 8 KB; use a shorter question or diagnostic header"
        );
    }
    Ok(())
}

pub struct RetrievalReport {
    pub results: Vec<SearchResult>,
    pub invalid: Vec<InvalidDocument>,
}

/// The single retrieval entry point used by search, ask, explain, and chat.
pub fn retrieve(project: &Path, query: &str) -> Result<RetrievalReport> {
    validate_query(query)?;
    let loaded = load_all_documents(project)?;
    let documents = loaded
        .documents
        .into_iter()
        .filter(|document| document.effective)
        .collect();
    Ok(RetrievalReport {
        results: KnowledgeIndex::build(documents).search(query),
        invalid: loaded.invalid,
    })
}
