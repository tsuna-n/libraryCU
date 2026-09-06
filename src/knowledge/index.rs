use std::{cmp::Reverse, collections::HashMap};

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
const ENGLISH_MARKER: &str = "<!-- lbc:en -->";
const THAI_MARKER: &str = "<!-- lbc:th -->";

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
    #[serde(skip_serializing)]
    pub document: KnowledgeDocument,
    pub source_id: String,
    pub title: String,
    pub kind: String,
    pub verification_status: String,
    pub source_locator: String,
    pub score: u32,
    pub match_reason: String,
    pub excerpt: String,
    pub matched_terms: usize,
    pub query_terms: usize,
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
                add_lookup_value(&mut index.by_tag, tag, position);
            }
            for keyword in &metadata.keywords {
                add_lookup_value(&mut index.by_keyword, keyword, position);
            }
            if let Some(category) = metadata.category.as_deref() {
                add_lookup_value(&mut index.by_category, category, position);
            }
            if let Some(tool) = metadata.tool.as_deref() {
                add_lookup_value(&mut index.by_tool, tool, position);
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
        let language = if query.chars().any(is_thai) {
            "th"
        } else {
            "en"
        };
        self.search_with_language(query, language)
    }

    pub fn search_with_language(&self, query: &str, language: &str) -> Vec<SearchResult> {
        let normalized = query.trim().to_lowercase();
        if normalized.is_empty() {
            return Vec::new();
        }
        let terms = query_terms(&normalized);

        let mut scores = vec![0u32; self.documents.len()];
        let mut matched_terms = vec![vec![false; terms.len()]; self.documents.len()];
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
                && (code.to_lowercase() == normalized || terms.contains(&code.to_lowercase()))
            {
                credit(position, CODE_WEIGHT, CODE_WEIGHT, "exact error code");
            }
            if self.titles[position].contains(&normalized) {
                credit(position, TITLE_WEIGHT, TITLE_WEIGHT, "title match");
            }
            for (term_index, term) in terms.iter().enumerate() {
                if self.titles[position].contains(term.as_str()) {
                    matched_terms[position][term_index] = true;
                    credit(
                        position,
                        TERM_TITLE_WEIGHT,
                        TERM_TITLE_WEIGHT,
                        "keyword match",
                    );
                }
                if self.bodies[position].contains(term.as_str()) {
                    matched_terms[position][term_index] = true;
                    let hits = self.bodies[position].matches(term.as_str()).count() as u32;
                    credit(
                        position,
                        TERM_BODY_WEIGHT * hits.min(MAX_BODY_HITS),
                        TERM_BODY_WEIGHT,
                        "keyword match",
                    );
                }
            }
        }
        for (term_index, key) in terms.iter().enumerate() {
            for positions in [
                self.by_tag.get(key),
                self.by_keyword.get(key),
                self.by_category.get(key),
                self.by_tool.get(key),
            ]
            .into_iter()
            .flatten()
            {
                for &position in positions {
                    matched_terms[position][term_index] = true;
                    credit(position, METADATA_WEIGHT, METADATA_WEIGHT, "metadata match");
                }
            }
        }

        for (position, hits) in matched_terms.iter().enumerate() {
            scores[position] += hits.iter().filter(|hit| **hit).count() as u32 * 25;
        }

        let mut results: Vec<SearchResult> = self
            .documents
            .iter()
            .enumerate()
            .filter(|(position, _)| scores[*position] > 0)
            .map(|(position, document)| {
                let mut localized = document.clone();
                localized.title = localized_title(document, language);
                localized.body = localized_body(&document.body, language).to_owned();
                SearchResult {
                    document: localized.clone(),
                    source_id: localized.source_id.clone(),
                    title: localized.title.clone(),
                    kind: localized.kind.clone(),
                    verification_status: localized.verification_status.clone(),
                    source_locator: localized.path.clone(),
                    score: scores[position],
                    match_reason: reasons[position].to_owned(),
                    excerpt: make_excerpt(&localized.body, &terms),
                    matched_terms: matched_terms[position].iter().filter(|hit| **hit).count(),
                    query_terms: terms.len(),
                }
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

fn localized_title(document: &KnowledgeDocument, language: &str) -> String {
    if language == "th" {
        document
            .metadata
            .title_th
            .clone()
            .filter(|title| !title.trim().is_empty())
            .unwrap_or_else(|| document.title.clone())
    } else {
        document.title.clone()
    }
}

fn localized_body<'a>(body: &'a str, language: &str) -> &'a str {
    let Some((_, after_english)) = body.split_once(ENGLISH_MARKER) else {
        return body;
    };
    let Some((english, thai)) = after_english.split_once(THAI_MARKER) else {
        return body;
    };
    if language == "th" {
        thai.trim()
    } else {
        english.trim()
    }
}

fn add_lookup_value(map: &mut HashMap<String, Vec<usize>>, value: &str, position: usize) {
    let normalized = value.to_lowercase();
    map.entry(normalized.clone()).or_default().push(position);
    if normalized.chars().any(is_thai) {
        for fragment in query_terms(&normalized) {
            if fragment != normalized {
                map.entry(fragment).or_default().push(position);
            }
        }
    }
}

fn query_terms(normalized: &str) -> Vec<String> {
    let mut terms = Vec::new();
    for raw_term in normalized.split(|character: char| !character.is_alphanumeric()) {
        let term = trim_thai_question_words(raw_term);
        if term.chars().count() <= 1 || is_stopword(term) {
            continue;
        }
        if term.chars().any(is_thai) && term.chars().count() > 4 {
            let characters: Vec<_> = term.chars().collect();
            for window in characters.windows(3) {
                let fragment: String = window.iter().collect();
                if !terms.contains(&fragment) {
                    terms.push(fragment);
                }
            }
        } else {
            terms.push(term.to_owned());
        }
    }
    terms
}

fn trim_thai_question_words(mut term: &str) -> &str {
    for prefix in ["แก้ปัญหา", "วิธีแก้", "ช่วยแก้"] {
        if let Some(rest) = term.strip_prefix(prefix) {
            term = rest;
            break;
        }
    }
    for suffix in ["อย่างไร", "ยังไง"] {
        if let Some(rest) = term.strip_suffix(suffix) {
            term = rest;
            break;
        }
    }
    term
}

fn is_thai(character: char) -> bool {
    ('\u{0E00}'..='\u{0E7F}').contains(&character)
}

fn is_stopword(term: &str) -> bool {
    matches!(
        term,
        "a" | "an"
            | "and"
            | "are"
            | "do"
            | "does"
            | "for"
            | "how"
            | "i"
            | "in"
            | "is"
            | "it"
            | "of"
            | "on"
            | "or"
            | "the"
            | "this"
            | "to"
            | "what"
            | "with"
            | "work"
            | "works"
            | "แก้"
            | "ปัญหา"
            | "วิธี"
            | "อย่างไร"
            | "ยังไง"
    )
}

fn make_excerpt(body: &str, terms: &[String]) -> String {
    // Detect multiline secrets before paragraph selection can discard their markers.
    let redacted = crate::security::redact_sensitive(body);
    let body = redacted.as_str();
    let paragraphs: Vec<_> = body
        .split("\n\n")
        .map(str::trim)
        .filter(|part| {
            !part.is_empty()
                && !part.starts_with('#')
                && !part.starts_with("```")
                && !part.starts_with("<!--")
        })
        .collect();
    let selected_index = paragraphs
        .iter()
        .enumerate()
        .max_by_key(|(index, paragraph)| {
            let lowered = paragraph.to_lowercase();
            (
                terms
                    .iter()
                    .map(|term| lowered.matches(term.as_str()).count())
                    .sum::<usize>(),
                Reverse(*index),
            )
        })
        .map(|(index, _)| index)
        .unwrap_or(0);
    let selected = paragraphs
        .get(selected_index)
        .copied()
        .unwrap_or_else(|| body.trim());
    let clean = selected
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    truncate_chars(&clean, 700)
}

fn truncate_chars(input: &str, limit: usize) -> String {
    if input.chars().count() <= limit {
        return input.to_owned();
    }
    let mut value: String = input.chars().take(limit).collect();
    value.push('…');
    value
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
            source: "test".to_owned(),
            source_id: format!("test:{id}"),
            kind: "note".to_owned(),
            verification_status: "unverified".to_owned(),
            writable: false,
            effective: true,
            overridden_by: None,
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

    #[test]
    fn bilingual_document_uses_the_requested_language() {
        let body = "<!-- lbc:en -->\n# Port conflict\n\nThe port is already in use.\n\n<!-- lbc:th -->\n# พอร์ตชน\n\nพอร์ตนี้ถูกใช้งานอยู่แล้ว";
        let index = KnowledgeIndex::build(vec![document(
            "port",
            "Port conflict",
            body,
            KnowledgeMetadata {
                title_th: Some("พอร์ตถูกใช้งานอยู่แล้ว".to_owned()),
                keywords: vec!["port".to_owned(), "พอร์ต".to_owned()],
                ..Default::default()
            },
        )]);

        let english = index.search_with_language("port", "en");
        assert_eq!(english[0].title, "Port conflict");
        assert!(english[0].excerpt.contains("already in use"));
        assert!(!english[0].excerpt.contains("พอร์ต"));

        let thai = index.search_with_language("พอร์ต", "th");
        assert_eq!(thai[0].title, "พอร์ตถูกใช้งานอยู่แล้ว");
        assert!(thai[0].excerpt.contains("ถูกใช้งาน"));
        assert!(!thai[0].excerpt.contains("already in use"));
    }
}
