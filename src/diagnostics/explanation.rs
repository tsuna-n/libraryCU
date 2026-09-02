use std::path::Path;

use anyhow::{Context, Result};
use serde::Serialize;

use crate::{
    config::settings::ScannerConfig,
    knowledge,
    scanner::{ProjectInfo, scan_project},
};

use super::{
    diagnostic::Diagnostic,
    parser::parse_primary,
    rules::{RuleOutcome, apply_rule},
};

pub use super::rules::Confidence;

#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeReference {
    pub title: String,
    pub path: String,
    pub match_reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExplanationReport {
    pub project: ProjectInfo,
    pub diagnostic: Diagnostic,
    pub evidence: Vec<String>,
    pub cause: String,
    pub suggested_fixes: Vec<String>,
    pub verification: Vec<String>,
    pub next_steps: Vec<String>,
    pub knowledge: Vec<KnowledgeReference>,
    pub confidence: Confidence,
    pub files_inspected: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai: Option<crate::ai::AiContribution>,
}

pub fn explain(
    input: &str,
    project_path: &Path,
    scanner_config: &ScannerConfig,
) -> Result<ExplanationReport> {
    let diagnostic = parse_primary(input).context("no error input was provided")?;
    let scan = scan_project(project_path, scanner_config)?;
    let query = diagnostic.code.as_deref().unwrap_or(&diagnostic.message);
    let index = knowledge::KnowledgeIndex::build(knowledge::load_documents(&scan.project.root)?);
    let matches = index.search(query);
    let RuleOutcome {
        mut evidence,
        cause,
        suggested_fixes,
        verification,
        next_steps,
        confidence,
    } = apply_rule(&diagnostic, &scan.project.root, input);
    if !scan.project.languages.is_empty() {
        evidence.push(format!(
            "Project detection found {}.",
            scan.project.stack_label()
        ));
    }
    evidence.push(format!(
        "The safe scanner inspected {} project files.",
        scan.files_inspected
    ));
    let knowledge = matches
        .into_iter()
        .take(3)
        .map(|result| KnowledgeReference {
            title: result.document.title,
            path: result.document.path,
            match_reason: result.match_reason,
        })
        .collect();

    Ok(ExplanationReport {
        project: scan.project,
        diagnostic,
        evidence,
        cause,
        suggested_fixes,
        verification,
        next_steps,
        knowledge,
        confidence,
        files_inspected: scan.files_inspected,
        ai: None,
    })
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::*;

    #[test]
    fn unknown_error_remains_unknown() -> Result<()> {
        let report = explain(
            "connection terminated unexpectedly",
            Path::new(env!("CARGO_MANIFEST_DIR")),
            &ScannerConfig::default(),
        )?;
        assert_eq!(report.confidence, Confidence::Unknown);
        assert!(report.suggested_fixes.is_empty());
        assert!(!report.next_steps.is_empty());
        Ok(())
    }

    #[test]
    fn moved_value_uses_a_known_rule_and_knowledge() -> Result<()> {
        let report = explain(
            "error[E0382]: borrow of moved value: `name`\n --> src/main.rs:10:5\n value moved here",
            Path::new(env!("CARGO_MANIFEST_DIR")),
            &ScannerConfig::default(),
        )?;
        assert_eq!(report.confidence, Confidence::KnownRule);
        assert!(report.cause.contains("ownership"));
        assert!(
            report
                .knowledge
                .iter()
                .any(|item| item.path.ends_with("E0382.md"))
        );
        Ok(())
    }
}
