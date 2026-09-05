use anyhow::Result;

use crate::config::settings::AiConfig;
use crate::diagnostics::ExplanationReport;

use super::{
    AiClient, AiContribution, AiResponse, build_request, parse_confidence, resolve_client,
    strip_confidence_marker,
};

/// Extend a deterministic report with an AI analysis.
///
/// Callers are expected to fall back to the deterministic report when this
/// fails; AI is an optional layer, never a required one.
pub fn enhance(report: &mut ExplanationReport, redacted_input: &str, ai: &AiConfig) -> Result<()> {
    enhance_with_language(report, redacted_input, ai, "en")
}

pub fn enhance_with_language(
    report: &mut ExplanationReport,
    redacted_input: &str,
    ai: &AiConfig,
    language: &str,
) -> Result<()> {
    let client = resolve_client(ai)?;
    let request =
        super::context::build_request_with_language(report, redacted_input, &ai.model, language);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| anyhow::anyhow!("failed to start the async runtime: {error}"))?;
    let response = runtime.block_on(client.chat(request))?;
    apply_response(report, response, client.name());
    Ok(())
}

/// Record a provider response on the report, parsing its stated confidence.
pub fn apply_response(report: &mut ExplanationReport, response: AiResponse, provider: &str) {
    let confidence = parse_confidence(&response.content).unwrap_or("unspecified");
    report.ai = Some(AiContribution {
        provider: provider.to_owned(),
        model: response.model,
        analysis: strip_confidence_marker(&response.content),
        confidence: confidence.to_owned(),
    });
}

/// Used by tests and callers that already hold a resolved client.
pub fn enhance_with_client(
    report: &mut ExplanationReport,
    redacted_input: &str,
    client: &AiClient,
    model: &str,
) -> Result<()> {
    let request = build_request(report, redacted_input, model);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let response = runtime.block_on(client.chat(request))?;
    apply_response(report, response, client.name());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics;

    fn sample_report() -> ExplanationReport {
        diagnostics::explain(
            "error[E0382]: borrow of moved value: `name`\n --> src/main.rs:10:5\n value moved here",
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")),
            &crate::config::settings::ScannerConfig::default(),
        )
        .expect("sample diagnostic should explain")
    }

    #[test]
    fn records_an_ai_contribution_with_parsed_confidence() {
        let mut report = sample_report();
        apply_response(
            &mut report,
            AiResponse {
                content:
                    "The value was moved.\n\nSuggested fix: pass a reference.\n\nConfidence: medium"
                        .to_owned(),
                model: "model-x".to_owned(),
            },
            "openrouter",
        );
        let ai = report.ai.expect("AI contribution should be recorded");
        assert_eq!(ai.provider, "openrouter");
        assert_eq!(ai.model, "model-x");
        assert_eq!(ai.confidence, "medium");
        assert!(!ai.analysis.contains("Confidence:"));
        assert!(ai.analysis.contains("pass a reference"));
    }

    #[test]
    fn reports_unspecified_confidence_when_the_marker_is_missing() {
        let mut report = sample_report();
        apply_response(
            &mut report,
            AiResponse {
                content: "Just an analysis.".to_owned(),
                model: "model-x".to_owned(),
            },
            "openai-compat",
        );
        assert_eq!(
            report.ai.expect("AI contribution").confidence,
            "unspecified"
        );
    }
}
