use std::path::Path;

use anyhow::Result;

use super::{InvalidDocument, KnowledgeIndex, SearchResult, load_all_documents};

pub struct RetrievalReport {
    pub results: Vec<SearchResult>,
    pub invalid: Vec<InvalidDocument>,
}

/// The single retrieval entry point used by search, ask, explain, and chat.
pub fn retrieve(project: &Path, query: &str) -> Result<RetrievalReport> {
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
