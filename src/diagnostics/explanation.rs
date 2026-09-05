use std::path::Path;

use anyhow::{Context, Result};
use serde::Serialize;

use crate::{
    config::settings::ScannerConfig,
    knowledge,
    scanner::{EvidenceExcerpt, ProjectInfo, collect_diagnostic_evidence, scan_project},
};

use super::{
    diagnostic::Diagnostic,
    rules::{RuleOutcome, apply_rule},
};

pub use super::rules::Confidence;

#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeReference {
    pub source_id: String,
    pub title: String,
    pub path: String,
    pub match_reason: String,
    pub excerpt: String,
    pub score: u32,
    pub verification_status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExplanationReport {
    pub project: ProjectInfo,
    pub diagnostic: Diagnostic,
    pub diagnostics_detected: usize,
    pub evidence: Vec<String>,
    pub cause: String,
    pub suggested_fixes: Vec<String>,
    pub verification: Vec<String>,
    pub next_steps: Vec<String>,
    pub knowledge: Vec<KnowledgeReference>,
    pub confidence: Confidence,
    pub files_inspected: usize,
    pub project_evidence: Vec<EvidenceExcerpt>,
    pub warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai: Option<crate::ai::AiContribution>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_error: Option<String>,
}

pub fn explain(
    input: &str,
    project_path: &Path,
    scanner_config: &ScannerConfig,
) -> Result<ExplanationReport> {
    let diagnostics = super::parser::parse_diagnostics(input);
    let diagnostic = diagnostics
        .first()
        .cloned()
        .context("no error input was provided")?;
    let scan = scan_project(project_path, scanner_config)?;
    let query = diagnostic
        .code
        .as_ref()
        .map(|code| format!("{code} {}", diagnostic.message))
        .unwrap_or_else(|| diagnostic.message.clone());
    let retrieved = knowledge::retrieve(&scan.project.root, &query)?;
    let matches: Vec<_> = retrieved
        .results
        .into_iter()
        .filter(crate::answer::is_adequate)
        .collect();
    let RuleOutcome {
        mut evidence,
        mut cause,
        mut suggested_fixes,
        verification,
        next_steps,
        mut confidence,
    } = apply_rule(&diagnostic, &scan.project.root, input);
    if !scan.project.languages.is_empty() {
        evidence.push(format!(
            "Project detection found {}.",
            scan.project.stack_label()
        ));
    }
    evidence.push(format!("The inventory scan counted {} eligible project files; this does not mean their contents were analyzed.", scan.files_inspected));
    if diagnostics.len() > 1 {
        evidence.push(format!("{} diagnostics were detected; this report explains the first and leaves {} additional diagnostic(s) unexpanded.", diagnostics.len(), diagnostics.len() - 1));
    }
    let (project_evidence, mut warnings) =
        collect_diagnostic_evidence(&scan.project.root, &diagnostic, scanner_config);
    evidence.push(format!(
        "{} bounded project file excerpt(s) supplied direct content evidence; project detection read {} small manifest(s).",
        project_evidence.len(), scan.file_contents_read
    ));
    let knowledge = matches
        .into_iter()
        .take(3)
        .map(|result| KnowledgeReference {
            source_id: result.document.source_id,
            title: result.document.title,
            path: result.document.path,
            match_reason: result.match_reason,
            excerpt: result.excerpt,
            score: result.score,
            verification_status: result.document.verification_status,
        })
        .collect::<Vec<_>>();
    if confidence == Confidence::Unknown && !knowledge.is_empty() {
        cause = "No hardcoded diagnostic rule matched. The guidance below comes from retrieved local knowledge and has not been verified against this project.".to_owned();
        suggested_fixes = knowledge
            .iter()
            .map(|item| format!("[{}] {}", item.source_id, item.excerpt))
            .collect();
        confidence = Confidence::RetrievedKnowledge;
    }
    warnings.extend(scan.warnings.clone());
    warnings.extend(
        retrieved
            .invalid
            .into_iter()
            .map(|invalid| format!("{}: {}", invalid.path, invalid.error)),
    );

    Ok(ExplanationReport {
        project: scan.project,
        diagnostic,
        diagnostics_detected: diagnostics.len(),
        evidence,
        cause,
        suggested_fixes,
        verification,
        next_steps,
        knowledge,
        confidence,
        files_inspected: scan.files_inspected,
        project_evidence,
        warnings,
        ai: None,
        ai_error: None,
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
