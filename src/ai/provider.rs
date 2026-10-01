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

/// Bound all SSE wire bytes (including hidden reasoning/framing), decode only
/// complete UTF-8 lines, and stop reading immediately at the completion marker.
pub(crate) async fn read_chat_stream(
    mut response: reqwest::Response,
    model: &str,
    on_event: &mut (dyn FnMut(StreamEvent) + Send),
) -> anyhow::Result<AiResponse> {
    let mut decoder = StreamDecoder::default();
    let mut redactor = StreamRedactor::default();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| anyhow::anyhow!("failed to read provider stream: {}", e.without_url()))?
    {
        decoder.push(&chunk, on_event, &mut redactor)?;
        if decoder.done {
            break;
        }
    }
    decoder.finish(on_event, &mut redactor)?;
    let content = crate::security::redact_sensitive(decoder.content.trim());
    if content.is_empty() {
        anyhow::bail!(if decoder.thinking {
            "the reasoning model spent its entire token budget on hidden reasoning and returned no answer"
        } else {
            "provider response contained no assistant content"
        });
    }
    redactor.flush(true, on_event);
    Ok(AiResponse {
        content,
        model: crate::security::redact_sensitive(model),
    })
}

#[derive(Default)]
struct StreamDecoder {
    pending: Vec<u8>,
    searched: usize,
    wire_bytes: usize,
    content: String,
    thinking: bool,
    done: bool,
}

impl StreamDecoder {
    fn push(
        &mut self,
        chunk: &[u8],
        callback: &mut (dyn FnMut(StreamEvent) + Send),
        redactor: &mut StreamRedactor,
    ) -> anyhow::Result<()> {
        if self.done {
            return Ok(());
        }
        anyhow::ensure!(
            self.wire_bytes.saturating_add(chunk.len()) <= MAX_PROVIDER_RESPONSE_BYTES,
            "provider stream exceeds 2 MB (including reasoning and framing)"
        );
        self.wire_bytes += chunk.len();
        self.pending.extend_from_slice(chunk);
        let mut consumed = 0;
        let mut start = self.searched;
        while let Some(relative) = self.pending[start..].iter().position(|byte| *byte == b'\n') {
            let end = start + relative;
            let line = std::str::from_utf8(&self.pending[consumed..end])
                .map_err(|_| anyhow::anyhow!("provider stream was not valid UTF-8"))?;
            let event = parse_sse_line(line.trim_end_matches('\r'))?;
            self.event(event, callback, redactor);
            consumed = end + 1;
            start = consumed;
            if self.done {
                break;
            }
        }
        self.pending.drain(..consumed);
        self.searched = self.pending.len();
        if self.done {
            self.pending.clear();
            self.searched = 0;
        }
        Ok(())
    }

    fn finish(
        &mut self,
        callback: &mut (dyn FnMut(StreamEvent) + Send),
        redactor: &mut StreamRedactor,
    ) -> anyhow::Result<()> {
        if !self.done && !self.pending.is_empty() {
            let line = std::str::from_utf8(&self.pending)
                .map_err(|_| anyhow::anyhow!("provider stream ended with invalid UTF-8"))?;
            let event = parse_sse_line(line.trim_end_matches(['\r', '\n']))?;
            self.event(event, callback, redactor);
            self.pending.clear();
        }
        Ok(())
    }

    fn event(
        &mut self,
        event: SseLine,
        callback: &mut (dyn FnMut(StreamEvent) + Send),
        redactor: &mut StreamRedactor,
    ) {
        match event {
            SseLine::Done => self.done = true,
            SseLine::Delta { content, reasoning } => {
                if reasoning.is_some() && !self.thinking {
                    self.thinking = true;
                    callback(StreamEvent::Thinking);
                }
                if let Some(text) = content {
                    self.content.push_str(&text);
                    redactor.raw.push_str(&text);
                    if text.contains('\n') {
                        redactor.flush(false, callback);
                    }
                }
            }
            SseLine::Empty => {}
        }
    }
}

/// Hold incomplete logical lines so split credentials cannot escape redaction.
/// Private-key state survives lines without repeatedly rescanning the response.
#[derive(Default)]
struct StreamRedactor {
    raw: String,
    private_key: bool,
}

impl StreamRedactor {
    fn flush(&mut self, final_chunk: bool, callback: &mut (dyn FnMut(StreamEvent) + Send)) {
        let end = if final_chunk {
            self.raw.len()
        } else {
            self.raw.rfind('\n').map(|end| end + 1).unwrap_or(0)
        };
        let ready: String = self.raw.drain(..end).collect();
        for line in ready.split_inclusive('\n') {
            let safe = if self.private_key {
                let contextual = crate::security::redact_sensitive(&format!(
                    "-----BEGIN PRIVATE KEY-----\n{line}"
                ));
                contextual
                    .strip_prefix("[REDACTED PRIVATE KEY]\n")
                    .expect("private-key context is always redacted")
                    .to_owned()
            } else {
                crate::security::redact_sensitive(line)
            };
            let lower = line.to_ascii_lowercase();
            if lower.contains("-----begin") && lower.contains("private key-----") {
                self.private_key = true;
            }
            if self.private_key && lower.contains("-----end") && lower.contains("private key-----")
            {
                self.private_key = false;
            }
            callback(StreamEvent::Content(&safe));
        }
    }
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
    fn sse_decoder_preserves_every_utf8_byte_split_and_stops_at_done() {
        let payload = "data: {\"choices\":[{\"delta\":{\"content\":\"งานไทย 🦀\\n\"}}]}\r\n\r\ndata: [DONE]\n";
        for split in 0..=payload.len() {
            let mut decoder = StreamDecoder::default();
            let mut redactor = StreamRedactor::default();
            let mut visible = String::new();
            let mut callback = |event: StreamEvent<'_>| {
                if let StreamEvent::Content(text) = event {
                    visible.push_str(text);
                }
            };
            decoder
                .push(&payload.as_bytes()[..split], &mut callback, &mut redactor)
                .unwrap();
            decoder
                .push(&payload.as_bytes()[split..], &mut callback, &mut redactor)
                .unwrap();
            decoder
                .push(b"not JSON after DONE\n", &mut callback, &mut redactor)
                .unwrap();
            decoder.finish(&mut callback, &mut redactor).unwrap();
            redactor.flush(true, &mut callback);
            assert!(decoder.done);
            assert_eq!(decoder.content, "งานไทย 🦀\n");
            assert_eq!(visible, "งานไทย 🦀\n");
        }
    }

    #[test]
    fn sse_decoder_bounds_unfinished_lines_and_reasoning_wire_bytes_at_limit() {
        for size in [
            MAX_PROVIDER_RESPONSE_BYTES - 1,
            MAX_PROVIDER_RESPONSE_BYTES,
            MAX_PROVIDER_RESPONSE_BYTES + 1,
        ] {
            let mut decoder = StreamDecoder::default();
            let mut redactor = StreamRedactor::default();
            let mut callback = |_: StreamEvent<'_>| {};
            assert_eq!(
                decoder
                    .push(&vec![b' '; size], &mut callback, &mut redactor)
                    .is_ok(),
                size <= MAX_PROVIDER_RESPONSE_BYTES
            );
            if size == MAX_PROVIDER_RESPONSE_BYTES {
                assert!(decoder.push(b"x", &mut callback, &mut redactor).is_err());
                assert_eq!(decoder.pending.len(), MAX_PROVIDER_RESPONSE_BYTES);
            }
        }
        let line = b"data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"hidden\"}}]}\n";
        let mut decoder = StreamDecoder::default();
        let mut redactor = StreamRedactor::default();
        let mut callback = |_: StreamEvent<'_>| {};
        for _ in 0..MAX_PROVIDER_RESPONSE_BYTES / line.len() {
            decoder.push(line, &mut callback, &mut redactor).unwrap();
        }
        assert!(decoder.push(line, &mut callback, &mut redactor).is_err());
        assert!(decoder.content.is_empty());
        assert!(decoder.thinking);
    }

    #[test]
    fn sse_decoder_rejects_invalid_utf8_and_parses_final_line_without_newline() {
        let mut decoder = StreamDecoder::default();
        let mut redactor = StreamRedactor::default();
        let mut callback = |_: StreamEvent<'_>| {};
        assert!(
            decoder
                .push(b"data: \xff\n", &mut callback, &mut redactor)
                .is_err()
        );
        let mut decoder = StreamDecoder::default();
        decoder
            .push(
                b"data: {\"choices\":[{\"delta\":{\"content\":\"ok\"}}]}",
                &mut callback,
                &mut redactor,
            )
            .unwrap();
        decoder.finish(&mut callback, &mut redactor).unwrap();
        assert_eq!(decoder.content, "ok");
        let mut decoder = StreamDecoder::default();
        decoder.push(&[0xe0], &mut callback, &mut redactor).unwrap();
        assert!(decoder.finish(&mut callback, &mut redactor).is_err());
    }

    #[test]
    fn streamed_secrets_are_redacted_before_callbacks_even_across_content_deltas() {
        let mut redactor = StreamRedactor::default();
        let mut visible = String::new();
        let mut callback = |event: StreamEvent<'_>| {
            if let StreamEvent::Content(text) = event {
                visible.push_str(text);
            }
        };
        for fragment in [
            "Change: x\nAPI_",
            "KEY=private-stream-marker",
            "\n-----BEGIN PRIVATE KEY-----\n",
            "private-interior-marker\n",
            "-----END PRIVATE KEY-----\nDone",
        ] {
            redactor.raw.push_str(fragment);
            redactor.flush(false, &mut callback);
        }
        redactor.flush(true, &mut callback);
        assert!(visible.contains("Change: x"));
        assert!(visible.ends_with("Done"));
        assert!(!visible.contains("private-stream-marker"));
        assert!(!visible.contains("private-interior-marker"));
        assert!(visible.contains("[REDACTED PRIVATE KEY]"));
    }

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
