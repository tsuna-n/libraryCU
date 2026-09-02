use serde_json::json;

use super::openai_compat::OpenAiCompatProvider;
use super::openrouter::OpenRouterProvider;

/// Marker the model is instructed to end its answer with.
pub const CONFIDENCE_MARKER: &str = "Confidence:";

#[derive(Debug, Clone)]
pub struct AiRequest {
    pub system: String,
    pub user: String,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

#[derive(Debug, Clone)]
pub struct AiResponse {
    pub content: String,
    pub model: String,
}

/// Vendor-neutral abstraction; core logic must depend on this trait only.
pub trait AiProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn chat(
        &self,
        request: AiRequest,
    ) -> impl std::future::Future<Output = anyhow::Result<AiResponse>> + Send;
}

pub enum AiClient {
    OpenRouter(OpenRouterProvider),
    OpenAiCompat(OpenAiCompatProvider),
}

impl AiClient {
    pub fn name(&self) -> &'static str {
        match self {
            Self::OpenRouter(provider) => provider.name(),
            Self::OpenAiCompat(provider) => provider.name(),
        }
    }

    pub async fn chat(&self, request: AiRequest) -> anyhow::Result<AiResponse> {
        match self {
            Self::OpenRouter(provider) => provider.chat(request).await,
            Self::OpenAiCompat(provider) => provider.chat(request).await,
        }
    }
}

pub fn parse_confidence(content: &str) -> Option<&'static str> {
    let lowered = content.to_ascii_lowercase();
    let marker = CONFIDENCE_MARKER.to_ascii_lowercase();
    let position = lowered.rfind(&marker)?;
    let tail = lowered[position + marker.len()..].trim_start();
    let word: String = tail
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric())
        .collect();
    match word.as_str() {
        "high" => Some("high"),
        "medium" => Some("medium"),
        "low" => Some("low"),
        _ => None,
    }
}

pub fn strip_confidence_marker(content: &str) -> String {
    let lowered = content.to_ascii_lowercase();
    match lowered.rfind(&CONFIDENCE_MARKER.to_ascii_lowercase()) {
        Some(position) => content[..position].trim_end().to_owned(),
        None => content.trim().to_owned(),
    }
}

/// Build the OpenAI chat-completions JSON body shared by both providers.
pub fn build_chat_body(request: &AiRequest) -> serde_json::Value {
    json!({
        "model": request.model,
        "max_tokens": request.max_tokens,
        "temperature": request.temperature,
        "messages": [
            {"role": "system", "content": request.system},
            {"role": "user", "content": request.user},
        ],
    })
}

/// Parse an OpenAI-style chat-completions response payload.
pub fn parse_chat_response(payload: &str, fallback_model: &str) -> anyhow::Result<AiResponse> {
    let value: serde_json::Value = serde_json::from_str(payload)
        .map_err(|error| anyhow::anyhow!("provider returned invalid JSON: {error}"))?;
    if let Some(error) = value.get("error") {
        let message = error
            .get("message")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown provider error");
        anyhow::bail!("provider error: {message}");
    }
    let content = value
        .get("choices")
        .and_then(serde_json::Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(serde_json::Value::as_str)
        .filter(|content| !content.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("provider response contained no assistant content"))?;
    let model = value
        .get("model")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(fallback_model);
    Ok(AiResponse {
        content: content.to_owned(),
        model: model.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_stated_confidence_levels() {
        assert_eq!(
            parse_confidence("analysis\n\nConfidence: high"),
            Some("high")
        );
        assert_eq!(parse_confidence("confidence: MEDIUM"), Some("medium"));
        assert_eq!(parse_confidence("Confidence: low."), Some("low"));
        assert_eq!(parse_confidence("no marker here"), None);
        assert_eq!(parse_confidence("Confidence: probably"), None);
    }

    #[test]
    fn strips_the_confidence_marker_from_displayed_content() {
        let stripped = strip_confidence_marker("analysis text\n\nConfidence: high");
        assert_eq!(stripped, "analysis text");
        assert_eq!(strip_confidence_marker("plain"), "plain");
    }

    #[test]
    fn builds_a_chat_body_with_system_and_user_messages() {
        let request = AiRequest {
            system: "system prompt".to_owned(),
            user: "user prompt".to_owned(),
            model: "model-x".to_owned(),
            max_tokens: 512,
            temperature: 0.2,
        };
        let body = build_chat_body(&request);
        assert_eq!(body["model"], "model-x");
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["messages"][1]["content"], "user prompt");
    }

    #[test]
    fn parses_a_chat_completion_response() {
        let payload = r#"{
            "model": "model-x",
            "choices": [
                {"message": {"role": "assistant", "content": "analysis\n\nConfidence: high"}}
            ]
        }"#;
        let response = parse_chat_response(payload, "fallback").expect("response should parse");
        assert_eq!(response.model, "model-x");
        assert_eq!(parse_confidence(&response.content), Some("high"));
    }

    #[test]
    fn surfaces_provider_errors() {
        let payload = r#"{"error": {"message": "invalid api key"}}"#;
        let error = parse_chat_response(payload, "fallback").unwrap_err();
        assert!(error.to_string().contains("invalid api key"));
    }

    #[test]
    fn rejects_empty_content() {
        let payload = r#"{"choices": [{"message": {"content": "  "}}]}"#;
        assert!(parse_chat_response(payload, "fallback").is_err());
    }
}
