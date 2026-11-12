use std::{
    collections::{HashMap, HashSet},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Result;
use serde::Serialize;

use super::{KnowledgeDocument, LoadReport, loader};

#[derive(Debug, Clone, Serialize)]
pub struct ValidationWarning {
    pub code: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationReport {
    pub documents: usize,
    pub warnings: Vec<ValidationWarning>,
}

/// Inspect the corpus without updating documents, configuration, or the index.
pub fn validate(project: &Path, max_age_days: u64) -> Result<ValidationReport> {
    let report = loader::load_all_documents(project)?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    Ok(validate_report(&report, now, max_age_days))
}

/// Bare references must identify exactly one document across all loaded sources;
/// qualified references select `builtin:id`, `user:id`, `project:id`, or
/// `package:name:id`. Superseded documents remain valid reference targets.
pub fn validate_report(report: &LoadReport, now_unix: u64, max_age_days: u64) -> ValidationReport {
    let mut warnings = Vec::new();
    for invalid in &report.invalid {
        let code = if invalid.error.starts_with("duplicate ID") {
            "duplicate_id"
        } else if invalid.error.starts_with("override target")
            && invalid.error.ends_with("does not exist")
        {
            "missing_override"
        } else if invalid.error.starts_with("override target")
            && invalid.error.ends_with("is ambiguous")
        {
            "ambiguous_override"
        } else if invalid.error.contains("override cycle")
            || invalid.error.contains("cannot override itself")
        {
            "override_cycle"
        } else if invalid
            .error
            .starts_with("multiple equally-ranked overrides")
        {
            "conflicting_overrides"
        } else if invalid.error.starts_with("invalid override target") {
            "invalid_override"
        } else {
            "invalid_document"
        };
        warnings.push(ValidationWarning {
            code: code.to_owned(),
            path: invalid.path.clone(),
            source_id: report
                .documents
                .iter()
                .find(|document| document.path == invalid.path)
                .map(|document| document.source_id.clone()),
            message: crate::security::redact_sensitive(&invalid.error),
        });
    }
    let already_reported: HashSet<_> = warnings
        .iter()
        .map(|warning| (warning.path.clone(), warning.code.clone()))
        .collect();
    let mut qualified: HashMap<&str, Vec<&KnowledgeDocument>> = HashMap::new();
    let mut bare: HashMap<&str, Vec<&KnowledgeDocument>> = HashMap::new();
    for document in &report.documents {
        qualified
            .entry(&document.source_id)
            .or_default()
            .push(document);
        bare.entry(&document.metadata.id)
            .or_default()
            .push(document);
    }
    let mut warn = |document: &KnowledgeDocument, code: &str, message: String| {
        if !already_reported.contains(&(document.path.clone(), code.to_owned())) {
            warnings.push(ValidationWarning {
                code: code.to_owned(),
                path: document.path.clone(),
                source_id: Some(document.source_id.clone()),
                message: crate::security::redact_sensitive(&message),
            });
        }
    };
    for document in &report.documents {
        if qualified[document.source_id.as_str()].len() > 1 {
            warn(
                document,
                "duplicate_id",
                format!("duplicate source-qualified ID {}", document.source_id),
            );
        }
        for reference in &document.metadata.references {
            if !loader::valid_reference(reference, false) {
                warn(
                    document,
                    "invalid_reference",
                    format!(
                        "invalid knowledge reference {reference:?}; use an ID or source-qualified ID"
                    ),
                );
                continue;
            }
            let matches = if reference.contains(':') {
                qualified.get(reference.as_str())
            } else {
                bare.get(reference.as_str())
            };
            match matches.map(Vec::len).unwrap_or(0) {
                0 => warn(
                    document,
                    "missing_reference",
                    format!("reference {reference:?} does not exist"),
                ),
                1 => {}
                _ => warn(
                    document,
                    "ambiguous_reference",
                    format!(
                        "reference {reference:?} is ambiguous; use a unique source-qualified ID"
                    ),
                ),
            }
        }
        if let Some(target) = document.metadata.overrides.as_deref() {
            if !matches!(document.source.as_str(), "user" | "project")
                || !loader::valid_reference(target, true)
            {
                warn(
                    document,
                    "invalid_override",
                    "only user/project documents may override a valid source-qualified ID"
                        .to_owned(),
                );
            } else {
                match qualified.get(target).map(Vec::len).unwrap_or(0) {
                    0 => warn(
                        document,
                        "missing_override",
                        format!("override target {target:?} does not exist"),
                    ),
                    1 => {}
                    _ => warn(
                        document,
                        "ambiguous_override",
                        format!("override target {target:?} is ambiguous"),
                    ),
                }
            }
        }
        let created = document.metadata.created_at_unix;
        let updated = document.metadata.updated_at_unix;
        if created.is_none() || updated.is_none() {
            warn(
                document,
                "missing_dates",
                "missing creation or update date; review this document when convenient".to_owned(),
            );
        }
        if let (Some(created), Some(updated)) = (created, updated)
            && updated < created
        {
            warn(
                document,
                "inconsistent_dates",
                "update date precedes creation date".to_owned(),
            );
        }
        if created
            .into_iter()
            .chain(updated)
            .any(|date| date > now_unix)
        {
            warn(
                document,
                "future_dates",
                "creation or update date is in the future".to_owned(),
            );
        }
        if let Some(last_review) = updated.or(created)
            && now_unix.saturating_sub(last_review) > max_age_days.saturating_mul(86_400)
        {
            warn(
                document,
                "stale_document",
                format!(
                    "last recorded update is older than {max_age_days} days; review the document"
                ),
            );
        }
    }
    for warning in &mut warnings {
        warning.path = crate::security::redact_sensitive(&warning.path);
    }
    warnings.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.code.cmp(&right.code))
            .then(left.message.cmp(&right.message))
    });
    warnings.dedup_by(|left, right| {
        left.path == right.path && left.code == right.code && left.message == right.message
    });
    ValidationReport {
        documents: report.documents.len(),
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::loader::{InvalidDocument, parse_document_for_source};

    fn document(id: &str, source: &str, metadata: &str) -> super::super::KnowledgeDocument {
        parse_document_for_source(
            &format!("{id}.md"),
            &format!("---\nid: {id}\n{metadata}---\nA useful note.\n"),
            source,
            &format!("/{source}/{id}.md"),
            source == "user" || source == "project",
        )
        .unwrap()
    }

    fn codes(report: &ValidationReport) -> Vec<&str> {
        report
            .warnings
            .iter()
            .map(|warning| warning.code.as_str())
            .collect()
    }

    #[test]
    fn resolves_qualified_references_and_reports_bare_ambiguity_and_invalid_targets() {
        let report = LoadReport {
            documents: vec![
                document(
                    "topic",
                    "user",
                    "created_at_unix: 100\nupdated_at_unix: 200\n",
                ),
                document(
                    "topic",
                    "package:guide",
                    "created_at_unix: 100\nupdated_at_unix: 200\n",
                ),
                document(
                    "caller",
                    "project",
                    "created_at_unix: 100\nupdated_at_unix: 200\nreferences: [user:topic, 'package:guide:topic', topic, absent, 'https://example.com', 'package::topic']\n",
                ),
            ],
            invalid: Vec::new(),
        };
        let validation = validate_report(&report, 200, 365);
        assert_eq!(validation.documents, 3);
        assert_eq!(
            codes(&validation)
                .iter()
                .filter(|code| **code == "ambiguous_reference")
                .count(),
            1
        );
        assert_eq!(
            codes(&validation)
                .iter()
                .filter(|code| **code == "missing_reference")
                .count(),
            1
        );
        assert_eq!(
            codes(&validation)
                .iter()
                .filter(|code| **code == "invalid_reference")
                .count(),
            2
        );
        assert_eq!(validation.warnings.len(), 4);
    }

    #[test]
    fn warns_on_missing_stale_inconsistent_and_future_dates_without_underflow() {
        let now = 10 * 86_400;
        let report = LoadReport {
            documents: vec![
                document("legacy", "user", ""),
                document("stale", "user", "created_at_unix: 1\nupdated_at_unix: 2\n"),
                document(
                    "inconsistent",
                    "user",
                    "created_at_unix: 200\nupdated_at_unix: 100\n",
                ),
                document(
                    "future",
                    "user",
                    &format!(
                        "created_at_unix: {}\nupdated_at_unix: {}\n",
                        u64::MAX,
                        u64::MAX
                    ),
                ),
                document("created-only", "user", &format!("created_at_unix: {now}\n")),
            ],
            invalid: Vec::new(),
        };
        let validation = validate_report(&report, now, 1);
        assert_eq!(
            codes(&validation)
                .iter()
                .filter(|code| **code == "missing_dates")
                .count(),
            2
        );
        assert_eq!(
            codes(&validation)
                .iter()
                .filter(|code| **code == "stale_document")
                .count(),
            2
        );
        assert!(codes(&validation).contains(&"inconsistent_dates"));
        assert_eq!(
            codes(&validation)
                .iter()
                .filter(|code| **code == "future_dates")
                .count(),
            1
        );
        assert!(!codes(&validate_report(&report, now, u64::MAX)).contains(&"stale_document"));
    }

    #[test]
    fn reports_parse_failures_duplicate_ids_and_ambiguous_overrides() {
        let report = LoadReport {
            documents: vec![
                document("same", "user", ""),
                document("same", "user", "title: Different\n"),
                document("replacement", "project", "overrides: user:same\n"),
                document("invalid", "project", "overrides: same\n"),
                document("missing", "project", "overrides: user:absent\n"),
            ],
            invalid: vec![InvalidDocument {
                path: "/password=fixture-path-secret/invalid.md".to_owned(),
                error: "invalid YAML frontmatter; password=fixture-secret".to_owned(),
            }],
        };
        let validation = validate_report(&report, 100, 365);
        for code in [
            "invalid_document",
            "duplicate_id",
            "ambiguous_override",
            "invalid_override",
            "missing_override",
        ] {
            assert!(codes(&validation).contains(&code), "missing {code}");
        }
        assert!(
            !serde_json::to_string(&validation)
                .unwrap()
                .contains("fixture-secret")
        );
        assert!(
            !serde_json::to_string(&validation)
                .unwrap()
                .contains("fixture-path-secret")
        );
        assert_eq!(serde_json::to_value(&validation).unwrap()["documents"], 5);
    }
}
