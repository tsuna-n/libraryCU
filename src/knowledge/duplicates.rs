use std::collections::HashSet;

use super::{KnowledgeDocument, KnowledgeMetadata};

/// Offline suggestions only; callers decide whether to edit or add a note.
pub fn find_similar<'a>(
    documents: &'a [KnowledgeDocument],
    title: &str,
    problem: &str,
    metadata: &KnowledgeMetadata,
) -> Vec<&'a KnowledgeDocument> {
    let title = normalize(title);
    let problem = normalize(problem);
    let mut matches: Vec<_> = documents
        .iter()
        .filter(|document| document.effective)
        .filter_map(|document| {
            let existing_title = normalize(&document.title);
            let existing_problem =
                normalize(full_problem(&document.body).unwrap_or(&document.body));
            let exact = (!title.is_empty() && title == existing_title)
                || (!problem.is_empty() && problem == existing_problem);
            let similarity =
                overlap(&title, &existing_title).max(overlap(&problem, &existing_problem));
            let same_code = metadata.error_code.as_deref().is_some_and(|code| {
                !code.trim().is_empty()
                    && document
                        .metadata
                        .error_code
                        .as_deref()
                        .is_some_and(|existing| code.trim().eq_ignore_ascii_case(existing.trim()))
                    && match (
                        metadata.language.as_deref(),
                        document.metadata.language.as_deref(),
                    ) {
                        (Some(left), Some(right))
                            if !left.trim().is_empty() && !right.trim().is_empty() =>
                        {
                            left.trim().eq_ignore_ascii_case(right.trim())
                        }
                        _ => true,
                    }
            });
            let rank = if exact {
                3
            } else if similarity >= 70 {
                2
            } else if same_code {
                1
            } else {
                return None;
            };
            Some((document, rank, if rank == 2 { similarity } else { 0 }))
        })
        .collect();
    matches.sort_by(
        |(left, left_rank, left_similarity), (right, right_rank, right_similarity)| {
            right_rank
                .cmp(left_rank)
                .then(right_similarity.cmp(left_similarity))
                .then(editable_priority(right).cmp(&editable_priority(left)))
                .then(left.source_id.cmp(&right.source_id))
                .then(left.path.cmp(&right.path))
        },
    );
    matches
        .into_iter()
        .take(5)
        .map(|(document, _, _)| document)
        .collect()
}

fn editable_priority(document: &KnowledgeDocument) -> u8 {
    match (document.writable, document.source.as_str()) {
        (true, "project") => 2,
        (true, "user") => 1,
        _ => 0,
    }
}

fn normalize(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .map(|character| {
            // Keep non-ASCII combining marks, including Thai vowel/tone marks.
            if character.is_alphanumeric()
                || (!character.is_ascii()
                    && !character.is_whitespace()
                    && !matches!(character, '\u{2000}'..='\u{206f}' | '\u{3000}'..='\u{303f}'))
            {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn overlap(left: &str, right: &str) -> usize {
    let words = |text: &str| {
        text.split_whitespace()
            .filter(|word| {
                word.chars().count() >= 3
                    && !matches!(
                        *word,
                        "the"
                            | "and"
                            | "for"
                            | "from"
                            | "with"
                            | "without"
                            | "when"
                            | "this"
                            | "that"
                            | "after"
                            | "before"
                            | "into"
                            | "cannot"
                            | "does"
                            | "not"
                            | "error"
                            | "failed"
                            | "failure"
                            | "problem"
                            | "issue"
                            | "using"
                    )
            })
            .map(str::to_owned)
            .collect::<HashSet<_>>()
    };
    // shortcut: whitespace tokens only; add language-aware segmentation if needed.
    let left = words(left);
    let right = words(right);
    let shared = left.intersection(&right).count();
    if shared < 2 {
        0
    } else {
        200 * shared / (left.len() + right.len())
    }
}

// Display details truncate at 1,000 characters; duplicate checks need the full section.
fn full_problem(body: &str) -> Option<&str> {
    let mut start = None;
    let mut offset = 0;
    let mut fence: Option<(char, usize)> = None;
    for line in body.split_inclusive('\n') {
        let trimmed = line.trim();
        if fence.is_none() {
            if trimmed.eq_ignore_ascii_case("## Problem") {
                start = Some(offset + line.len());
            } else if start.is_some() && (trimmed.starts_with("# ") || trimmed.starts_with("## ")) {
                return start.map(|start| body[start..offset].trim());
            }
        }
        if let Some(marker @ ('`' | '~')) = trimmed.chars().next() {
            let length = trimmed
                .chars()
                .take_while(|character| *character == marker)
                .count();
            if length >= 3 {
                match fence {
                    None => fence = Some((marker, length)),
                    Some((open, count))
                        if marker == open
                            && length >= count
                            && trimmed[length..].trim().is_empty() =>
                    {
                        fence = None
                    }
                    _ => {}
                }
            }
        }
        offset += line.len();
    }
    start.map(|start| body[start..].trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document(id: &str, title: &str, problem: &str) -> KnowledgeDocument {
        KnowledgeDocument {
            metadata: KnowledgeMetadata::default(),
            title: title.into(),
            body: format!("## Problem\n\n{problem}\n\n## Solution\n\nA recorded fix."),
            path: format!("{id}.md"),
            source: "project".into(),
            source_id: format!("project:{id}"),
            kind: "troubleshooting".into(),
            verification_status: "unverified".into(),
            writable: true,
            effective: true,
            overridden_by: None,
        }
    }

    #[test]
    fn matches_normalized_title_or_problem() {
        let documents = vec![
            document("title", "Borrow: moved VALUE!", "Different context"),
            document("problem", "A separate title", "Vector, allocation failed."),
        ];
        let matches = find_similar(
            &documents,
            "borrow moved value",
            "VECTOR allocation failed",
            &KnowledgeMetadata::default(),
        );
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn finds_reworded_problem_with_meaningful_overlap() {
        let documents = vec![document(
            "borrow",
            "Cannot compile application",
            "Rust vector borrow after moved value",
        )];
        let matches = find_similar(
            &documents,
            "Ownership diagnostic",
            "Borrow of moved value in Rust vector",
            &KnowledgeMetadata::default(),
        );
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn matches_error_code_only_when_languages_are_compatible() {
        let mut rust = document(
            "rust",
            "Rust compiler diagnostic",
            "Ownership transfers to receiver",
        );
        rust.metadata.language = Some("Rust".into());
        rust.metadata.error_code = Some("E0382".into());
        let mut python = document(
            "python",
            "Python worker issue",
            "Process cannot open socket",
        );
        python.metadata.language = Some("python".into());
        python.metadata.error_code = Some("E0382".into());
        let documents = vec![rust, python];
        let metadata = KnowledgeMetadata {
            language: Some("rust".into()),
            error_code: Some("e0382".into()),
            ..Default::default()
        };
        let matches = find_similar(
            &documents,
            "Unusual diagnostic",
            "An unrelated message",
            &metadata,
        );
        assert_eq!(
            matches
                .iter()
                .map(|document| document.source_id.as_str())
                .collect::<Vec<_>>(),
            ["project:rust"]
        );
    }

    #[test]
    fn ignores_unrelated_and_single_generic_shared_words() {
        let documents = vec![
            document("network", "Connection error", "Network socket unavailable"),
            document("database", "Database error", "Failed to open database"),
        ];
        assert!(
            find_similar(
                &documents,
                "Compiler error",
                "Failed to compile application",
                &KnowledgeMetadata::default()
            )
            .is_empty()
        );
    }

    #[test]
    fn includes_read_only_matches_and_ignores_ineffective_documents() {
        let mut builtin = document(
            "builtin",
            "Port already occupied",
            "Listener cannot bind address",
        );
        builtin.source = "builtin".into();
        builtin.source_id = "builtin:port".into();
        builtin.writable = false;
        let mut hidden = builtin.clone();
        hidden.source_id = "project:hidden".into();
        hidden.effective = false;
        let documents = vec![hidden, builtin];
        let matches = find_similar(
            &documents,
            "Port already occupied",
            "A different description",
            &KnowledgeMetadata::default(),
        );
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].source_id, "builtin:port");
        assert!(!matches[0].writable);
    }

    #[test]
    fn ranks_exact_then_similarity_then_code_and_bounds_deterministically() {
        let mut documents = vec![
            document(
                "similar",
                "Reworded title",
                "Borrow of moved value from vector",
            ),
            document("code", "Other title", "Cannot create a socket"),
        ];
        documents[1].metadata.error_code = Some("E0382".into());
        for id in ["z", "c", "b", "a", "d"] {
            documents.push(document(id, "Moved value", "Separate description"));
        }
        let mut builtin = document("builtin", "Moved value", "Separate description");
        builtin.source_id = "builtin:aaa".into();
        builtin.source = "builtin".into();
        builtin.writable = false;
        documents.push(builtin);
        let metadata = KnowledgeMetadata {
            error_code: Some("E0382".into()),
            ..Default::default()
        };
        let ids = |documents: &[KnowledgeDocument]| {
            find_similar(
                documents,
                "Moved value",
                "Vector borrow after moved value",
                &metadata,
            )
            .iter()
            .map(|document| document.source_id.clone())
            .collect::<Vec<_>>()
        };
        assert_eq!(
            ids(&documents),
            [
                "project:a",
                "project:b",
                "project:c",
                "project:d",
                "project:z"
            ]
        );
        documents.reverse();
        assert_eq!(
            ids(&documents),
            [
                "project:a",
                "project:b",
                "project:c",
                "project:d",
                "project:z"
            ]
        );
        documents.retain(|document| {
            document.source_id == "project:a"
                || document.source_id == "project:similar"
                || document.source_id == "project:code"
        });
        assert_eq!(
            ids(&documents),
            ["project:a", "project:similar", "project:code"]
        );
    }

    #[test]
    fn compares_the_entire_problem_instead_of_the_display_excerpt() {
        let prefix = "the ".repeat(400);
        let documents = vec![document(
            "long",
            "Database connections",
            &format!("{prefix}database connection refused"),
        )];
        assert!(
            find_similar(
                &documents,
                "Compiler diagnostics",
                &format!("{prefix}compiler ownership mismatch"),
                &KnowledgeMetadata::default()
            )
            .is_empty()
        );
        assert_eq!(
            find_similar(
                &documents,
                "Compiler diagnostics",
                &format!("{prefix}database connection refused"),
                &KnowledgeMetadata::default()
            )
            .len(),
            1
        );
    }

    #[test]
    fn preserves_thai_marks_and_does_not_guess_word_boundaries() {
        let documents = vec![document("thai", "แก้ไขปัญหา", "เปิดไฟล์ไม่ได้")];
        assert_eq!(
            find_similar(
                &documents,
                "แก้ไขปัญหา!",
                "ต่างกัน",
                &KnowledgeMetadata::default()
            )
            .len(),
            1
        );
        assert!(
            find_similar(
                &documents,
                "แกไขปัญหา",
                "เปิดไฟล์ได้",
                &KnowledgeMetadata::default()
            )
            .is_empty()
        );
    }
}
