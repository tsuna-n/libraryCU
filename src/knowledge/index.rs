use std::collections::HashMap;

use serde::Serialize;

use super::document::KnowledgeDocument;

/// Weight for an exact error-code hit; dominates all other signals.
const CODE_WEIGHT: u32 = 1_000;
/// Weight for a full-query title hit.
const TITLE_WEIGHT: u32 = 100;
/// Weight for an exact tag, keyword, category, or tool hit.
const METADATA_WEIGHT: u32 = 80;
/// Per-term weights, applied for each query term.
const TERM_TITLE_WEIGHT: u32 = 20;
const TERM_BODY_WEIGHT: u32 = 3;
/// Maximum body occurrences counted per term.
const MAX_BODY_HITS: u32 = 5;

/// Precomputed lookup structures over the knowledge store.
///
/// Building the index normalizes keys once so repeated searches avoid
/// re-lowering every document field on every query.
pub struct KnowledgeIndex {
    documents: Vec<KnowledgeDocument>,
    by_error_code: HashMap<String, Vec<usize>>,
    by_tag: HashMap<String, Vec<usize>>,
    by_keyword: HashMap<String, Vec<usize>>,
    by_category: HashMap<String, Vec<usize>>,
    by_tool: HashMap<String, Vec<usize>>,
    titles: Vec<String>,
    bodies: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub document: KnowledgeDocument,
    pub score: u32,
    pub match_reason: String,
}

impl KnowledgeIndex {
    pub fn build(documents: Vec<KnowledgeDocument>) -> Self {
        let count = documents.len();
        let mut index = Self {
            documents,
            by_error_code: HashMap::new(),
            by_tag: HashMap::new(),
            by_keyword: HashMap::new(),
            by_category: HashMap::new(),
            by_tool: HashMap::new(),
            titles: Vec::with_capacity(count),
            bodies: Vec::with_capacity(count),
        };
        for (position, document) in index.documents.iter().enumerate() {
            let metadata = &document.metadata;
            if let Some(code) = metadata.error_code.as_deref() {
                index
                    .by_error_code
                    .entry(code.to_lowercase())
                    .or_default()
                    .push(position);
            }
            for tag in &metadata.tags {
                index
                    .by_tag
                    .entry(tag.to_lowercase())
                    .or_default()
                    .push(position);
            }
            for keyword in &metadata.keywords {
                index
                    .by_keyword
                    .entry(keyword.to_lowercase())
                    .or_default()
                    .push(position);
            }
            if let Some(category) = metadata.category.as_deref() {
                index
                    .by_category
                    .entry(category.to_lowercase())
                    .or_default()
                    .push(position);
            }
            if let Some(tool) = metadata.tool.as_deref() {
                index
                    .by_tool
                    .entry(tool.to_lowercase())
                    .or_default()
                    .push(position);
            }
            index.titles.push(document.title.to_lowercase());
            index.bodies.push(document.body.to_lowercase());
        }
        index
    }

    pub fn len(&self) -> usize {
        self.documents.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    /// Exact error-code lookup, the strongest retrieval signal.
    pub fn lookup_error_code(&self, code: &str) -> Vec<&KnowledgeDocument> {
        self.by_error_code
            .get(&code.trim().to_lowercase())
            .map(|positions| {
                positions
                    .iter()
                    .map(|&position| &self.documents[position])
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        let normalized = query.trim().to_lowercase();
        if normalized.is_empty() {
            return Vec::new();
        }
        let terms: Vec<&str> = normalized
            .split(|character: char| !character.is_alphanumeric())
            .filter(|term| !term.is_empty())
            .collect();

        let mut scores = vec![0u32; self.documents.len()];
        let mut reasons = vec!["keyword match"; self.documents.len()];
        let mut best_signal = vec![0u32; self.documents.len()];

        // `signal` orders which match is reported: the strongest signal seen wins.
        let mut credit = |position: usize, score: u32, signal: u32, reason: &'static str| {
            scores[position] += score;
            if signal > best_signal[position] {
                best_signal[position] = signal;
                reasons[position] = reason;
            }
        };

        for (position, document) in self.documents.iter().enumerate() {
            if let Some(code) = &document.metadata.error_code
                && code.to_lowercase() == normalized
            {
                credit(position, CODE_WEIGHT, CODE_WEIGHT, "exact error code");
            }
            if self.titles[position].contains(&normalized) {
                credit(position, TITLE_WEIGHT, TITLE_WEIGHT, "title match");
            }
            for term in &terms {
                if self.titles[position].contains(term) {
                    credit(
                        position,
                        TERM_TITLE_WEIGHT,
                        TERM_TITLE_WEIGHT,
                        "keyword match",
                    );
                }
                if self.bodies[position].contains(term) {
                    let hits = self.bodies[position].matches(term).count() as u32;
                    credit(
                        position,
                        TERM_BODY_WEIGHT * hits.min(MAX_BODY_HITS),
                        TERM_BODY_WEIGHT,
                        "keyword match",
                    );
                }
            }
        }
        for key in &terms {
            for positions in [
                self.by_tag.get(*key),
                self.by_keyword.get(*key),
                self.by_category.get(*key),
                self.by_tool.get(*key),
            ]
            .into_iter()
            .flatten()
            {
                for &position in positions {
                    credit(position, METADATA_WEIGHT, METADATA_WEIGHT, "metadata match");
                }
            }
        }

        let mut results: Vec<SearchResult> = self
            .documents
            .iter()
            .enumerate()
            .filter(|(position, _)| scores[*position] > 0)
            .map(|(position, document)| SearchResult {
                document: document.clone(),
                score: scores[position],
                match_reason: reasons[position].to_owned(),
            })
            .collect();
        results.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then_with(|| left.document.title.cmp(&right.document.title))
        });
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::KnowledgeMetadata;

    fn document(
        id: &str,
        title: &str,
        body: &str,
        metadata: KnowledgeMetadata,
    ) -> KnowledgeDocument {
        KnowledgeDocument {
            metadata: KnowledgeMetadata {
                id: id.to_owned(),
                ..metadata
            },
            title: title.to_owned(),
            body: body.to_owned(),
            path: format!("{id}.md"),
        }
    }

    #[test]
    fn exact_error_code_dominates_and_is_reported() {
        let index = KnowledgeIndex::build(vec![
            document(
                "a",
                "Borrowing guide",
                "This guide discusses E0382 and moved values at length, E0382 again.",
                KnowledgeMetadata::default(),
            ),
            document(
                "b",
                "E0382 diagnostic",
                "Use of moved value.",
                KnowledgeMetadata {
                    error_code: Some("E0382".to_owned()),
                    ..Default::default()
                },
            ),
        ]);
        let results = index.search("e0382");
        assert_eq!(results.first().expect("result").document.metadata.id, "b");
        assert_eq!(
            results.first().expect("result").match_reason,
            "exact error code"
        );
        let lookup = index.lookup_error_code("E0382");
        assert_eq!(lookup.len(), 1);
        assert_eq!(lookup[0].metadata.id, "b");
    }

    #[test]
    fn metadata_matches_rank_above_plain_keyword_hits() {
        let index = KnowledgeIndex::build(vec![
            document(
                "body-only",
                "Unrelated title",
                "merge conflict markers appear once here",
                KnowledgeMetadata::default(),
            ),
            document(
                "tagged",
                "Unrelated title too",
                "No terms here.",
                KnowledgeMetadata {
                    tags: vec!["merge".to_owned()],
                    ..Default::default()
                },
            ),
        ]);
        let results = index.search("merge");
        assert_eq!(
            results.first().expect("result").document.metadata.id,
            "tagged"
        );
        assert_eq!(
            results.first().expect("result").match_reason,
            "metadata match"
        );
    }

    #[test]
    fn body_term_frequency_breaks_ties() {
        let index = KnowledgeIndex::build(vec![
            document(
                "sparse",
                "Cache guide",
                "cache eviction",
                KnowledgeMetadata::default(),
            ),
            document(
                "rich",
                "Cache guide two",
                "cache cache cache cache cache cache cache",
                KnowledgeMetadata::default(),
            ),
        ]);
        let results = index.search("cache");
        assert_eq!(results[0].document.metadata.id, "rich");
        assert!(results[0].score > results[1].score);
    }

    #[test]
    fn empty_query_returns_no_results() {
        let index =
            KnowledgeIndex::build(vec![document("a", "T", "B", KnowledgeMetadata::default())]);
        assert!(index.search("").is_empty());
        assert!(index.search("   ").is_empty());
    }
}
