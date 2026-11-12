use std::{collections::HashSet, path::Path};

use anyhow::Result;

use super::{InvalidDocument, KnowledgeIndex, SearchResult};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum RetrievalScope {
    #[default]
    All,
    Project,
}

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
    let language = if query
        .chars()
        .any(|character| ('\u{0E00}'..='\u{0E7F}').contains(&character))
    {
        "th"
    } else {
        "en"
    };
    retrieve_with_language(project, query, language)
}

/// Retrieve knowledge and render localized built-in passages in `language`.
pub fn retrieve_with_language(
    project: &Path,
    query: &str,
    language: &str,
) -> Result<RetrievalReport> {
    retrieve_with_scope(project, query, language, RetrievalScope::All)
}

pub fn retrieve_with_scope(
    project: &Path,
    query: &str,
    language: &str,
    scope: RetrievalScope,
) -> Result<RetrievalReport> {
    validate_query(query)?;
    retrieve_documents(project, query, language, None, scope)
}

/// Explain's reader already bounds the log; retain its larger input contract.
pub(crate) fn retrieve_diagnostic(
    project: &Path,
    input: &str,
    diagnostic: &crate::diagnostics::Diagnostic,
) -> Result<RetrievalReport> {
    retrieve_diagnostic_with_scope(project, input, diagnostic, RetrievalScope::All)
}

pub(crate) fn retrieve_diagnostic_with_scope(
    project: &Path,
    input: &str,
    diagnostic: &crate::diagnostics::Diagnostic,
    scope: RetrievalScope,
) -> Result<RetrievalReport> {
    let language = if input
        .chars()
        .any(|c| ('\u{0e00}'..='\u{0e7f}').contains(&c))
    {
        "th"
    } else {
        "en"
    };
    retrieve_documents(project, input, language, Some(diagnostic), scope)
}

fn retrieve_documents(
    project: &Path,
    query: &str,
    language: &str,
    diagnostic: Option<&crate::diagnostics::Diagnostic>,
    scope: RetrievalScope,
) -> Result<RetrievalReport> {
    let project = crate::scanner::detect_project(project)?;
    let loaded = super::loader::load_all_documents_with_scope(&project.root, scope)?;
    let documents = loaded
        .documents
        .into_iter()
        .filter(|document| document.effective)
        .collect();
    Ok(RetrievalReport {
        results: search_index(
            &KnowledgeIndex::build(documents),
            query,
            language,
            diagnostic,
            &project,
        )?,
        invalid: loaded.invalid,
    })
}

fn search_index(
    index: &KnowledgeIndex,
    query: &str,
    language: &str,
    diagnostic: Option<&crate::diagnostics::Diagnostic>,
    project: &crate::scanner::ProjectInfo,
) -> Result<Vec<SearchResult>> {
    let parsed = if diagnostic.is_none() {
        crate::diagnostics::parse_diagnostics(query)
    } else {
        Vec::new()
    };
    let diagnostics: Vec<_> = if let Some(diagnostic) = diagnostic {
        vec![diagnostic]
    } else {
        parsed
            .iter()
            .filter(|diagnostic| {
                diagnostic.source.is_some()
                    || diagnostic.code.is_some()
                    || diagnostic.file.is_some()
            })
            .collect()
    };
    if diagnostics.is_empty() || (diagnostic.is_none() && index.has_explicit_identifier(query)) {
        validate_query(query)?;
        return Ok(select_results(
            query,
            index.search_with_project(query, language, project),
        ));
    }
    let mut combined = Vec::new();
    let mut seen = HashSet::new();
    for diagnostic in diagnostics {
        let source = match diagnostic.source.as_deref() {
            Some("go") => "Go",
            Some(source) => source,
            None => "",
        };
        let focused = format!(
            "{} {source} {}",
            diagnostic.code.as_deref().unwrap_or_default(),
            diagnostic.message
        );
        validate_query(&focused)?;
        let mut results = index.search_with_project(&focused, language, project);
        if let Some(code) = &diagnostic.code {
            let code = if diagnostic.source.as_deref() == Some("python") {
                code.rsplit('.').next().unwrap_or(code)
            } else {
                code.as_str()
            };
            // Removing log noise must not turn an unsupported exception into a known fix.
            if !results.iter().any(|result| supports_code(result, code)) {
                results.retain(|result| {
                    result.match_reason == super::index::PYTHON_ERROR_FALLBACK_REASON
                });
            }
        }
        for result in &mut results {
            result.ranking_reasons.push("diagnostic message".to_owned());
        }
        combined.extend(
            select_results(&focused, results)
                .into_iter()
                .filter(|result| seen.insert(result.source_id.clone())),
        );
    }
    Ok(combined)
}

fn supports_code(result: &SearchResult, code: &str) -> bool {
    let code = code.to_lowercase();
    result
        .document
        .metadata
        .error_code
        .as_deref()
        .is_some_and(|value| value.eq_ignore_ascii_case(&code))
        || result
            .document
            .metadata
            .keywords
            .iter()
            .chain(&result.document.metadata.tags)
            .any(|value| super::index::contains_term(&value.to_lowercase(), &code))
        || super::index::contains_term(&result.document.title.to_lowercase(), &code)
        || super::index::contains_term(&result.document.body.to_lowercase(), &code)
}

pub fn is_adequate(result: &SearchResult) -> bool {
    result.match_reason != super::index::PYTHON_ERROR_FALLBACK_REASON
        && (matches!(
            result.match_reason.as_str(),
            "exact error code" | "exact identifier" | "title match"
        ) || (result.query_terms > 0 && result.matched_terms * 3 >= result.query_terms * 2))
}

fn select_results(query: &str, mut results: Vec<SearchResult>) -> Vec<SearchResult> {
    let normalized = query.trim().to_lowercase();
    let known_codes: Vec<_> = results
        .iter()
        .filter_map(|result| result.document.metadata.error_code.as_ref())
        .map(|code| code.to_lowercase())
        .collect();
    let required_codes: Vec<_> = normalized
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|word| {
            word.starts_with("err_")
                || known_codes.iter().any(|code| {
                    word.strip_prefix(code)
                        .is_some_and(|suffix| suffix.starts_with('_'))
                })
                || ["e", "ts"].iter().any(|prefix| {
                    word.strip_prefix(prefix).is_some_and(|digits| {
                        digits.len() >= 4 && digits.bytes().all(|c| c.is_ascii_digit())
                    })
                })
        })
        .collect();
    let matches_id = |result: &SearchResult| {
        super::index::contains_identifier(&normalized, &result.source_id.to_lowercase())
            || normalized == result.document.metadata.id.to_lowercase()
    };
    let explicit_id = results.iter().any(&matches_id);
    let exact_codes: Vec<_> = results
        .iter()
        .filter(|r| r.match_reason == "exact error code")
        .filter_map(|r| r.document.metadata.error_code.as_ref())
        .map(|code| code.to_lowercase())
        .collect();
    results.retain(|result| {
        if result.match_reason == super::index::PYTHON_ERROR_FALLBACK_REASON {
            return true;
        }
        if explicit_id {
            return matches_id(result);
        }
        let title = result.document.title.to_lowercase();
        let body = result.document.body.to_lowercase();
        is_adequate(result)
            && (result.match_reason == "exact error code"
                || required_codes.is_empty()
                || required_codes
                    .iter()
                    .any(|code| supports_code(result, code)))
            && (exact_codes.is_empty()
                || result.match_reason == "exact error code"
                || exact_codes.iter().any(|code| {
                    super::index::contains_term(&title, code)
                        || (result.document.metadata.error_code.is_none()
                            && super::index::contains_term(&body, code))
                })
                // A broad exception class can have a more specific untyped symptom note.
                || (result.document.metadata.error_code.is_none()
                    && exact_codes
                        .iter()
                        .any(|code| code.ends_with("error") || code.ends_with("exception"))
                    && result
                        .document
                        .metadata
                        .keywords
                        .iter()
                        .chain(&result.document.metadata.tags)
                        .any(|phrase| {
                            phrase.split_whitespace().count() > 1
                                && super::index::contains_term(&normalized, &phrase.to_lowercase())
                        })))
    });
    if !explicit_id && exact_codes.is_empty() {
        let mut phrases: Vec<_> = results
            .iter()
            .filter(|result| is_adequate(result))
            .flat_map(|result| {
                result
                    .document
                    .metadata
                    .keywords
                    .iter()
                    .chain(&result.document.metadata.tags)
            })
            .filter(|phrase| {
                phrase.contains(' ')
                    || phrase
                        .chars()
                        .any(|c| ('\u{0E00}'..='\u{0E7F}').contains(&c))
            })
            .filter(|phrase| super::index::contains_term(&normalized, &phrase.to_lowercase()))
            .map(|phrase| phrase.to_lowercase())
            .collect();
        let longest_phrase = phrases
            .iter()
            .map(|phrase| phrase.chars().count())
            .max()
            .unwrap_or(0);
        phrases.retain(|phrase| phrase.chars().count() == longest_phrase);
        let lexical_score = |result: &SearchResult| {
            result.score
                - super::index::source_weight(&result.document.source)
                - super::index::project_context_weight(result)
        };
        let best_score = results
            .iter()
            .filter(|r| is_adequate(r))
            .map(&lexical_score)
            .max()
            .unwrap_or(0);
        results.retain(|result| {
            !is_adequate(result)
                || result.match_reason == "title match"
                || if longest_phrase > 0 {
                    // A recorded symptom is equally valid in Markdown and metadata.
                    phrases.iter().any(|phrase| {
                        result
                            .document
                            .metadata
                            .keywords
                            .iter()
                            .chain(&result.document.metadata.tags)
                            .any(|value| value.to_lowercase() == *phrase)
                            || super::index::contains_term(
                                &result.document.title.to_lowercase(),
                                phrase,
                            )
                            || super::index::contains_term(
                                &result.document.body.to_lowercase(),
                                phrase,
                            )
                    })
                } else {
                    result.kind == "concept" || lexical_score(result) * 3 >= best_score * 2
                }
        });
    }
    let has_evidence = results.iter().any(is_adequate);
    results.retain(|result| !has_evidence || is_adequate(result));
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::loader::parse_document;

    fn index(notes: &[(&str, &str)]) -> KnowledgeIndex {
        KnowledgeIndex::build(
            notes
                .iter()
                .map(|(id, text)| parse_document(id, text).unwrap())
                .collect(),
        )
    }

    #[test]
    fn complete_symptom_phrase_excludes_broad_overlap() {
        let index = index(&[
            (
                "specific.md",
                "---\nid: specific\nkeywords: [partially initialized module]\n---\n# Circular import\n\nCannot import from a partially initialized module.",
            ),
            (
                "broad.md",
                "---\nid: broad\nkeywords: [cannot import name]\n---\n# Missing module\n\nCannot import name from module.",
            ),
        ]);
        let query = "cannot import name from partially initialized module";
        let results = select_results(query, index.search(query));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document.metadata.id, "specific");
    }

    #[test]
    fn unknown_codes_cannot_borrow_a_known_errors_message() {
        let index = index(&[(
            "known.md",
            "---\nid: known\nerror_code: ERR_MODULE_NOT_FOUND\n---\n# Node module\n\nCannot find package.",
        )]);
        for query in [
            "ERR_MODULE_NOT_FOUND_EXTRA cannot find package",
            "TS9999 cannot find package",
            "E03080 cannot find package",
            "EADDRINUSE_EXTRA cannot find package",
            "eaddrinuse_extra cannot find package",
        ] {
            assert!(
                select_results(query, index.search(query)).is_empty(),
                "{query}"
            );
        }
    }

    #[test]
    fn code_titled_notes_and_multiple_exact_codes_remain_available() {
        let index = index(&[
            (
                "first.md",
                "---\nid: first\nerror_code: E0308\n---\n# Mismatched types\n\nBorrow a string.",
            ),
            (
                "second.md",
                "---\nid: second\nerror_code: E0382\n---\n# Moved value\n\nBorrow the value.",
            ),
            (
                "local.md",
                "---\nid: local\n---\n# E0308 local solution\n\nBorrow a string.",
            ),
        ]);
        let query = "E0308 E0382";
        let results = select_results(query, index.search(query));
        assert_eq!(results.len(), 2);
        assert!(
            select_results("E0308", index.search("E0308"))
                .iter()
                .any(|r| r.document.metadata.id == "local")
        );
    }

    #[test]
    fn weak_supplements_are_removed_but_body_only_knowledge_still_works() {
        let notes = index(&[
            (
                "strong.md",
                "---\nid: strong\nkeywords: [recovery, cedar]\n---\n# Cedar recovery\n\nUse cedar recovery.",
            ),
            (
                "weak.md",
                "---\nid: weak\n---\n# Other workflow\n\nAn example mentions cedar recovery.",
            ),
        ]);
        let query = "cedar recovery";
        let results = select_results(query, notes.search(query));
        assert_eq!(results.len(), 1);
        let body_only = index(&[(
            "body.md",
            "---\nid: body\n---\n# Local note\n\nUse cedar recovery.",
        )]);
        assert_eq!(select_results(query, body_only.search(query)).len(), 1);
    }

    #[test]
    fn symptom_phrases_in_the_body_are_equal_evidence_to_metadata() {
        let notes = index(&[
            (
                "specific.md",
                "---\nid: specific\n---\n# Cedar recovery after crash\n\nCedar recovery after crash requires clearing the stale lock.",
            ),
            (
                "generic.md",
                "---\nid: generic\nkeywords: [after crash]\n---\n# Other workflow\n\nAn example mentions cedar recovery after crash.",
            ),
        ]);
        let query = "how to cedar recovery after crash";
        let results = select_results(query, notes.search(query));
        assert_eq!(results[0].document.metadata.id, "specific");
    }

    #[test]
    fn explicit_ids_survive_a_stronger_primary_code_signal() {
        let notes = index(&[
            (
                "first.md",
                "---\nid: first\nerror_code: E0308\n---\n# Local mismatch\n\nBorrow a string.",
            ),
            (
                "second.md",
                "---\nid: second\n---\n# Other note\n\nOther workflow.",
            ),
        ]);
        let query = "unknown:first unknown:second E0308";
        assert_eq!(select_results(query, notes.search(query)).len(), 2);
    }

    #[test]
    fn code_in_a_note_body_and_comparable_concepts_remain_searchable() {
        let notes = index(&[
            (
                "code.md",
                "---\nid: code\nerror_code: E0308\n---\n# Mismatched types\n\nBorrow a string.",
            ),
            (
                "body.md",
                "---\nid: body\n---\n# Local workaround\n\nE0308 requires borrowing this database value.",
            ),
            (
                "concept.md",
                "---\nid: concept\nkind: concept\ncategory: ownership\n---\n# Ownership basics\n\nOwnership lets you borrow a value.",
            ),
            (
                "details.md",
                "---\nid: details\nkeywords: [ownership, borrow]\n---\n# Ownership and borrow\n\nUse ownership and borrow a value.",
            ),
        ]);
        assert_eq!(select_results("E0308", notes.search("E0308")).len(), 2);
        assert!(
            select_results("ownership borrow", notes.search("ownership borrow"))
                .iter()
                .any(|r| r.document.metadata.id == "concept")
        );
    }

    #[test]
    fn ordinary_uppercase_words_are_not_error_codes() {
        let notes = index(&[(
            "moved.md",
            "---\nid: moved\nerror_code: E0382\n---\n# Use of moved value\n\nBorrow of moved value can be fixed by borrowing.",
        )]);
        for query in [
            "explain: borrow of moved value",
            "EXPLAIN: borrow of moved value",
            "ERRORS: borrow of moved value",
            "E0382 in my_module.rs",
        ] {
            assert_eq!(
                select_results(query, notes.search(query)).len(),
                1,
                "{query}"
            );
        }
    }

    #[test]
    fn python_variable_names_do_not_become_required_error_codes() {
        let notes = index(&[(
            "name.md",
            "---\nid: name\nkeywords: [NameError, is not defined]\n---\n# Python NameError\n\nA name is not defined. Declare the name before using it.",
        )]);
        let query = "NameError python name 'missing_name' is not defined";
        assert_eq!(select_results(query, notes.search(query)).len(), 1);
    }

    #[test]
    fn structured_diagnostics_ignore_wrappers_but_require_the_actual_code() {
        let notes = index(&[
            (
                "db.md",
                "---\nid: db\nlanguage: python\nkeywords: [driver.StorageError, transaction is busy]\n---\n# Transaction busy\n\nA driver.StorageError means the transaction is busy.",
            ),
            (
                "go.md",
                "---\nid: go\nlanguage: go\nkeywords: [undefined identifier]\n---\n# Go undefined identifier\n\nDeclare the identifier before use.",
            ),
        ]);
        for query in [
            "driver.StorageError: transaction is busy\n  File \"somewhere/task_queue.py\", line 324, in dispatch_request",
            "Traceback (most recent call last):\n  File \"D:\\jobs\\task_queue.py\", line 88, in dispatch\ndriver.StorageError: transaction is busy",
            "go build ./internal/dispatcher\ninternal/dispatcher/handler.go:242:12: undefined identifier",
        ] {
            assert_eq!(
                search_index(&notes, query, "en", None, &Default::default())
                    .unwrap()
                    .len(),
                1,
                "{query}"
            );
        }
        let unsupported = "Traceback (most recent call last):\n  File \"task_queue.py\", line 88\nMysteryError: transaction is busy";
        assert!(
            search_index(&notes, unsupported, "en", None, &Default::default())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn thai_complete_body_support_survives_without_partial_symptom_citations() {
        let notes = index(&[
            (
                "primary.md",
                "---\nid: primary\nlanguage: node\nkeywords: [หาไฟล์]\n---\n# Node หาไฟล์ไม่เจอ\n\nหาไฟล์ไม่เจอ ให้ตรวจ directory.",
            ),
            (
                "support.md",
                "---\nid: support\n---\n# Local workaround\n\nหาไฟล์ไม่เจอ ให้ตรวจ working directory.",
            ),
            (
                "other.md",
                "---\nid: other\nlanguage: linux\n---\n# Command lookup\n\nหาไฟล์ executable ที่ใช้ได้ แล้วลองตรวจ log เจอข้อผิดพลาดอื่น.",
            ),
        ]);
        let results =
            search_index(&notes, "Node หาไฟล์ไม่เจอ", "th", None, &Default::default()).unwrap();
        assert!(results.iter().any(|r| r.document.metadata.id == "primary"));
        assert!(results.iter().any(|r| r.document.metadata.id == "support"));
        assert!(!results.iter().any(|r| r.document.metadata.id == "other"));
    }

    #[test]
    fn short_english_body_support_and_unqualified_shared_codes_remain_eligible() {
        let notes = index(&[
            (
                "primary.md",
                "---\nid: primary\nlanguage: node\nkeywords: [missing file]\n---\n# Node missing file\n\nA missing file needs the correct directory.",
            ),
            (
                "support.md",
                "---\nid: support\n---\n# Local workaround\n\nFor a missing file, check the working directory.",
            ),
            (
                "node.md",
                "---\nid: node\nlanguage: node\nerror_code: EADDRINUSE\n---\n# Node port\n\nAddress already in use.",
            ),
            (
                "python.md",
                "---\nid: python\nlanguage: python\nerror_code: EADDRINUSE\n---\n# Python port\n\nAddress already in use.",
            ),
        ]);
        assert_eq!(
            search_index(&notes, "Node missing file", "en", None, &Default::default())
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            search_index(
                &notes,
                "EADDRINUSE address already in use",
                "en",
                None,
                &Default::default()
            )
            .unwrap()
            .len(),
            2
        );
    }

    #[test]
    fn qualified_python_exceptions_reuse_short_class_aliases() {
        let notes = index(&[(
            "decode.md",
            "---\nid: decode\nlanguage: python\nerror_code: DecodeError\n---\n# Decode input\n\nDecodeError means the input cannot be decoded.",
        )]);
        let query = "Traceback (most recent call last):\n  File \"decoder.py\", line 19\ncodec.parser.DecodeError: input cannot be decoded";
        let results = search_index(&notes, query, "en", None, &Default::default()).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document.metadata.id, "decode");
    }

    #[test]
    fn explicit_ids_and_unstructured_multiline_questions_keep_their_context() {
        let notes = index(&[(
            "manual.md",
            "---\nid: manual\n---\n# Local recipe\n\nRotate the token to recover the widget.",
        )]);
        let query = "error[E9999]: unknown error\nSee unknown:manual";
        assert_eq!(
            search_index(&notes, query, "en", None, &Default::default())
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            search_index(
                &notes,
                "rotate token\nrecover widget",
                "en",
                None,
                &Default::default()
            )
            .unwrap()
            .len(),
            1
        );
    }

    #[test]
    fn recognized_exception_classes_allow_specific_supporting_symptom_notes() {
        let notes = index(&[
            (
                "class.md",
                "---\nid: class\nlanguage: python\nerror_code: ImportError\n---\n# Python ImportError\n\nCheck the import.",
            ),
            (
                "specific.md",
                "---\nid: specific\nlanguage: python\ntags: [python]\nkeywords: [dependency initialization cycle]\n---\n# Dependency initialization cycle\n\nBreak the dependency initialization cycle.",
            ),
            (
                "different.md",
                "---\nid: different\nlanguage: python\nerror_code: 422\ntags: [python]\nkeywords: [dependency initialization cycle]\n---\n# Different diagnostic\n\nA dependency initialization cycle can occur here too.",
            ),
        ]);
        let input = "Traceback (most recent call last):\n  File \"startup.py\", line 12\nImportError: dependency initialization cycle";
        let results = search_index(&notes, input, "en", None, &Default::default()).unwrap();
        assert!(results.iter().any(|r| r.document.metadata.id == "specific"));
        assert!(
            !results
                .iter()
                .any(|r| r.document.metadata.id == "different")
        );
        assert!(
            search_index(
                &notes,
                &input.replace("ImportError", "UnknownCustomError"),
                "en",
                None,
                &Default::default(),
            )
            .unwrap()
            .is_empty()
        );
    }

    #[test]
    fn search_retains_evidence_for_each_diagnostic_and_explain_can_focus_the_primary() {
        let notes = index(&[
            (
                "first.md",
                "---\nid: first\nerror_code: E0308\n---\n# Mismatched types\n\nBorrow a string.",
            ),
            (
                "second.md",
                "---\nid: second\nerror_code: E0382\n---\n# Moved value\n\nBorrow the value.",
            ),
        ]);
        let input = "error[E0308]: mismatched types\n --> first.rs:2:3\nerror[E0382]: borrow of moved value\n --> second.rs:8:4";
        assert_eq!(
            search_index(&notes, input, "en", None, &Default::default())
                .unwrap()
                .len(),
            2
        );
        let primary = crate::diagnostics::parse_primary(input).unwrap();
        assert_eq!(
            search_index(&notes, input, "en", Some(&primary), &Default::default())
                .unwrap()
                .len(),
            1
        );
    }
}
