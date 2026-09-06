use serde_json::json;

use super::openai_compat::OpenAiCompatProvider;
use super::openrouter::OpenRouterProvider;

/// Marker the model is instructed to end its answer with.
pub const CONFIDENCE_MARKER: &str = "Confidence:";
pub const MAX_PROVIDER_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

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

pub async fn read_bounded_response(
    mut response: reqwest::Response,
) -> anyhow::Result<(reqwest::StatusCode, String)> {
    let status = response.status();
    if response
        .content_length()
        .is_some_and(|length| length > MAX_PROVIDER_RESPONSE_BYTES as u64)
    {
        anyhow::bail!("provider response is larger than 2 MB");
    }
    let mut payload = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|error| {
        anyhow::anyhow!("failed to read provider response: {}", error.without_url())
    })? {
        if payload.len().saturating_add(chunk.len()) > MAX_PROVIDER_RESPONSE_BYTES {
            anyhow::bail!("provider response is larger than 2 MB");
        }
        payload.extend_from_slice(&chunk);
    }
    let payload = String::from_utf8(payload)
        .map_err(|_| anyhow::anyhow!("provider response was not valid UTF-8"))?;
    Ok((status, payload))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamEvent<'a> {
    Thinking,
    Content(&'a str),
}

/// Forward user-visible streaming content while retaining the confidence marker
/// used internally to populate structured output.
pub(crate) struct VisibleResponseStream<'a> {
    callback: &'a mut (dyn FnMut(StreamEvent) + Send),
    pending: String,
    suppressing_metadata: bool,
}

impl<'a> VisibleResponseStream<'a> {
    pub(crate) fn new(callback: &'a mut (dyn FnMut(StreamEvent) + Send)) -> Self {
        Self {
            callback,
            pending: String::new(),
            suppressing_metadata: false,
        }
    }

    pub(crate) fn push(&mut self, event: StreamEvent<'_>) {
        match event {
            StreamEvent::Thinking => (self.callback)(StreamEvent::Thinking),
            StreamEvent::Content(text) if !self.suppressing_metadata => {
                self.pending.push_str(text);
                self.flush_available();
            }
            StreamEvent::Content(_) => {}
        }
    }

    pub(crate) fn finish(&mut self) {
        if !self.suppressing_metadata && !self.pending.is_empty() {
            (self.callback)(StreamEvent::Content(&self.pending));
            self.pending.clear();
        }
    }

    fn flush_available(&mut self) {
        let lowered = self.pending.to_ascii_lowercase();
        let marker = CONFIDENCE_MARKER.to_ascii_lowercase();
        if let Some(position) = lowered.find(&marker) {
            let visible = self.pending[..position].trim_end().to_owned();
            if !visible.is_empty() {
                (self.callback)(StreamEvent::Content(&visible));
            }
            self.pending.clear();
            self.suppressing_metadata = true;
            return;
        }

        // Retain enough trailing bytes to recognize a marker split across SSE
        // chunks, while continuing to display the rest of the answer live.
        let keep = CONFIDENCE_MARKER.len().saturating_sub(1);
        if self.pending.len() <= keep {
            return;
        }
        let mut split = self.pending.len() - keep;
        while !self.pending.is_char_boundary(split) {
            split -= 1;
        }
        if split == 0 {
            return;
        }
        let visible = self.pending[..split].to_owned();
        self.pending.drain(..split);
        (self.callback)(StreamEvent::Content(&visible));
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SseLine {
    Done,
    Delta {
        content: Option<String>,
        reasoning: Option<String>,
    },
    Empty,
}

pub fn parse_sse_line(line: &str) -> anyhow::Result<SseLine> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with(':') {
        return Ok(SseLine::Empty);
    }
    let data = match trimmed.strip_prefix("data:") {
        Some(rest) => rest.trim(),
        None => return Ok(SseLine::Empty),
    };
    if data == "[DONE]" {
        return Ok(SseLine::Done);
    }
    let value: serde_json::Value = serde_json::from_str(data)
        .map_err(|e| anyhow::anyhow!("provider returned invalid streaming JSON: {e}"))?;
    if let Some(error) = value.get("error") {
        let message = error
            .get("message")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown provider error");
        anyhow::bail!(
            "provider error: {}",
            crate::security::redact_sensitive(message)
        );
    }
    let choice = value
        .get("choices")
        .and_then(serde_json::Value::as_array)
        .and_then(|choices| choices.first());
    let delta = choice.and_then(|c| c.get("delta"));
    let content = delta
        .and_then(|d| d.get("content"))
        .and_then(serde_json::Value::as_str)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned);
    let reasoning = delta
        .and_then(|d| d.get("reasoning_content"))
        .and_then(serde_json::Value::as_str)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned);

    Ok(SseLine::Delta { content, reasoning })
}

/// Vendor-neutral abstraction; core logic must depend on this trait only.
pub trait AiProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn chat(
        &self,
        request: AiRequest,
    ) -> impl std::future::Future<Output = anyhow::Result<AiResponse>> + Send;

    fn chat_stream<'a>(
        &'a self,
        request: AiRequest,
        on_event: &'a mut (dyn FnMut(StreamEvent) + Send),
    ) -> impl std::future::Future<Output = anyhow::Result<AiResponse>> + Send {
        async move {
            let response = self.chat(request).await?;
            on_event(StreamEvent::Content(&response.content));
            Ok(response)
        }
    }
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

    pub async fn chat_stream<'a>(
        &'a self,
        request: AiRequest,
        on_event: &'a mut (dyn FnMut(StreamEvent) + Send),
    ) -> anyhow::Result<AiResponse> {
        match self {
            Self::OpenRouter(provider) => provider.chat_stream(request, on_event).await,
            Self::OpenAiCompat(provider) => provider.chat_stream(request, on_event).await,
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
    build_chat_body_streaming(request, false)
}

/// Build the OpenAI chat-completions JSON body with explicit streaming flag.
pub fn build_chat_body_streaming(request: &AiRequest, stream: bool) -> serde_json::Value {
    json!({
        "model": request.model,
        "max_tokens": request.max_tokens,
        "temperature": request.temperature,
        "stream": stream,
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
        anyhow::bail!(
            "provider error: {}",
            crate::security::redact_sensitive(message)
        );
    }
    let message = value
        .get("choices")
        .and_then(serde_json::Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .ok_or_else(|| anyhow::anyhow!("provider response contained no choices"))?;
    let content = message
        .get("content")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim();
    if content.is_empty() {
        if message
            .get("reasoning_content")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|reasoning| !reasoning.trim().is_empty())
        {
            anyhow::bail!(
                "the reasoning model spent its entire token budget on hidden reasoning and returned no answer"
            );
        }
        anyhow::bail!("provider response contained no assistant content");
    }
    let model = value
        .get("model")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(fallback_model);
    Ok(AiResponse {
        content: crate::security::redact_sensitive(content),
        model: crate::security::redact_sensitive(model),
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
    fn visible_stream_hides_a_confidence_marker_split_across_chunks() {
        let mut visible = String::new();
        let mut thinking = false;
        {
            let mut callback = |event: StreamEvent<'_>| match event {
                StreamEvent::Thinking => thinking = true,
                StreamEvent::Content(text) => visible.push_str(text),
            };
            let mut stream = VisibleResponseStream::new(&mut callback);
            stream.push(StreamEvent::Thinking);
            stream.push(StreamEvent::Content("Change: src/main.rs:10\nFrom: old\n"));
            stream.push(StreamEvent::Content("To: new\n\nConfi"));
            stream.push(StreamEvent::Content("dence: high"));
            stream.finish();
        }
        assert!(thinking);
        assert_eq!(visible, "Change: src/main.rs:10\nFrom: old\nTo: new");
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

    #[test]
    fn explains_reasoning_models_that_return_no_answer() {
        let payload = r#"{
            "choices": [{"finish_reason": "length", "message": {
                "content": "",
                "reasoning_content": "Let me think about this problem step by step..."
            }}]
        }"#;
        let error = parse_chat_response(payload, "fallback").unwrap_err();
        assert!(error.to_string().contains("hidden reasoning"));
    }

    #[test]
    fn parses_sse_data_lines() {
        let line = r#"data: {"choices":[{"delta":{"content":"hello "}}]}"#;
        assert_eq!(
            parse_sse_line(line).unwrap(),
            SseLine::Delta {
                content: Some("hello ".to_owned()),
                reasoning: None
            }
        );
        let done = "data: [DONE]";
        assert_eq!(parse_sse_line(done).unwrap(), SseLine::Done);
        let reasoning = r#"data: {"choices":[{"delta":{"reasoning_content":"thinking..."}}]}"#;
        assert_eq!(
            parse_sse_line(reasoning).unwrap(),
            SseLine::Delta {
                content: None,
                reasoning: Some("thinking...".to_owned())
            }
        );
        let empty = "  \n";
        assert_eq!(parse_sse_line(empty).unwrap(), SseLine::Empty);
        let comment = ": ping";
        assert_eq!(parse_sse_line(comment).unwrap(), SseLine::Empty);
    }
}
