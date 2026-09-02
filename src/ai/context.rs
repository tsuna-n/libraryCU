use super::provider::{AiRequest, CONFIDENCE_MARKER};

/// Cap the raw error text sent to a provider so prompts stay compact.
pub const MAX_ERROR_CONTEXT_CHARS: usize = 8_000;

const SYSTEM_PROMPT: &str = "\
You are LCU, a deterministic developer diagnostic assistant. \
You extend an offline analysis with reasoning; you never invent APIs, files, or commands. \
Ground every claim in the provided evidence. \
If the evidence is insufficient, say so plainly and set a low confidence. \
Answer with short sections: Cause, Suggested fix, Verification. \
End your answer with a final line in the exact form \
\"Confidence: high\", \"Confidence: medium\", or \"Confidence: low\".";

/// Build a compact AI request from the deterministic explanation report plus
/// the already-redacted error input. Knowledge is referenced by title and
/// path only; the full knowledge database is never sent.
pub fn build_request(
    report: &crate::diagnostics::ExplanationReport,
    redacted_input: &str,
    model: &str,
) -> AiRequest {
    let mut user = String::new();
    user.push_str("# Diagnostic\n");
    user.push_str(&format!(
        "source: {}\ncode: {}\nmessage: {}\n",
        report.diagnostic.source.as_deref().unwrap_or("unknown"),
        report.diagnostic.code.as_deref().unwrap_or("none"),
        report.diagnostic.message,
    ));
    user.push_str("\n# Project\n");
    user.push_str(&report.project.stack_label());
    user.push('\n');
    if !report.knowledge.is_empty() {
        user.push_str("\n# Local knowledge references\n");
        for item in &report.knowledge {
            user.push_str(&format!("- {} ({})\n", item.title, item.path));
        }
    }
    user.push_str("\n# Deterministic analysis\n");
    user.push_str(&format!("confidence: {}\n", report.confidence));
    user.push_str("evidence:\n");
    for item in &report.evidence {
        user.push_str(&format!("- {item}\n"));
    }
    user.push_str(&format!("cause: {}\n", report.cause));
    if !report.suggested_fixes.is_empty() {
        user.push_str("suggested fixes:\n");
        for fix in &report.suggested_fixes {
            user.push_str(&format!("- {fix}\n"));
        }
    }
    user.push_str("\n# Redacted error output\n");
    user.push_str(&truncate_chars(redacted_input, MAX_ERROR_CONTEXT_CHARS));
    user.push_str(&format!(
        "\n\nImprove or confirm the analysis above. End with {CONFIDENCE_MARKER} high, medium, or low."
    ));

    AiRequest {
        system: SYSTEM_PROMPT.to_owned(),
        user,
        model: model.to_owned(),
        max_tokens: 1_024,
        temperature: 0.2,
    }
}

fn truncate_chars(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_owned();
    }
    let mut truncated: String = input.chars().take(max_chars).collect();
    truncated.push_str("\n... [truncated]");
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_report() -> crate::diagnostics::ExplanationReport {
        crate::diagnostics::explain(
            "error[E0382]: borrow of moved value: `name`\n --> src/main.rs:10:5\n value moved here",
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")),
            &crate::config::settings::ScannerConfig::default(),
        )
        .expect("sample diagnostic should explain")
    }

    #[test]
    fn includes_diagnostic_project_and_evidence() {
        let report = sample_report();
        let request = build_request(&report, "raw error", "model-x");
        assert!(request.user.contains("E0382"));
        assert!(request.user.contains("# Project"));
        assert!(request.user.contains("# Deterministic analysis"));
        assert!(request.user.contains("raw error"));
        assert!(request.system.ends_with("\"Confidence: low\"."));
        assert_eq!(request.model, "model-x");
    }

    #[test]
    fn references_knowledge_by_title_only() {
        let report = sample_report();
        let request = build_request(&report, "raw error", "model-x");
        assert!(request.user.contains("# Local knowledge references"));
        assert!(request.user.contains("E0382"));
        assert!(
            !request.user.contains("# Code sample"),
            "only titles and paths are sent"
        );
    }

    #[test]
    fn truncates_long_error_input() {
        let report = sample_report();
        let long_input = "x".repeat(MAX_ERROR_CONTEXT_CHARS + 500);
        let request = build_request(&report, &long_input, "model-x");
        assert!(request.user.contains("... [truncated]"));
        let (_, raw) = request
            .user
            .split_once("# Redacted error output\n")
            .expect("context should contain the error section");
        assert!(raw.contains("... [truncated]"));
        assert!(
            raw.chars().count() <= MAX_ERROR_CONTEXT_CHARS + 200,
            "truncated input should stay close to the limit"
        );
    }
}
