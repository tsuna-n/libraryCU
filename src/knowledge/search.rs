use super::document::KnowledgeDocument;
use super::index::{KnowledgeIndex, SearchResult};

/// Convenience wrapper that builds an index for a one-off search.
pub fn search(documents: &[KnowledgeDocument], query: &str) -> Vec<SearchResult> {
    KnowledgeIndex::build(documents.to_vec()).search(query)
}
