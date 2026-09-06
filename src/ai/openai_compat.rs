use anyhow::Context;
use reqwest::Client;

use super::provider::{
    AiProvider, AiRequest, AiResponse, SseLine, StreamEvent, build_chat_body,
    build_chat_body_streaming, parse_chat_response, parse_sse_line, read_bounded_response,
};

/// Talks to any OpenAI-compatible endpoint such as a local Ollama server
/// (`http://localhost:11434/v1`), vLLM, LM Studio, or OpenAI itself.
pub struct OpenAiCompatProvider {
    base_url: String,
    api_key: Option<String>,
    http: Client,
}

impl OpenAiCompatProvider {
    pub fn new(base_url: String, api_key: Option<String>) -> Self {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(90))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            base_url: base_url.trim_end_matches('/').to_owned(),
            api_key,
            http,
        }
    }

    pub fn endpoint(&self) -> String {
        format!("{}/chat/completions", self.base_url)
    }
}

impl AiProvider for OpenAiCompatProvider {
    fn name(&self) -> &'static str {
        "openai-compat"
    }

    async fn chat(&self, request: AiRequest) -> anyhow::Result<AiResponse> {
        let mut http = self
            .http
            .post(self.endpoint())
            .timeout(std::time::Duration::from_secs(90));
        if let Some(api_key) = &self.api_key {
            http = http.bearer_auth(api_key);
        }
        let response = http
            .json(&build_chat_body(&request))
            .send()
            .await
            .map_err(reqwest::Error::without_url)
            .context("failed to reach the OpenAI-compatible endpoint")?;
        let (status, payload) = read_bounded_response(response).await?;
        if !status.is_success() {
            anyhow::bail!("provider request failed with status {status}");
        }
        parse_chat_response(&payload, &request.model)
    }

    async fn chat_stream<'a>(
        &'a self,
        request: AiRequest,
        on_event: &'a mut (dyn FnMut(StreamEvent) + Send),
    ) -> anyhow::Result<AiResponse> {
        let mut http = self
            .http
            .post(self.endpoint())
            .timeout(std::time::Duration::from_secs(90));
        if let Some(api_key) = &self.api_key {
            http = http.bearer_auth(api_key);
        }
        let mut response = http
            .json(&build_chat_body_streaming(&request, true))
            .send()
            .await
            .map_err(reqwest::Error::without_url)
            .context("failed to reach the OpenAI-compatible endpoint")?;
        let status = response.status();
        if !status.is_success() {
            let (_, payload) = read_bounded_response(response).await?;
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&payload)
                && let Some(msg) = value
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
            {
                anyhow::bail!("provider error: {}", crate::security::redact_sensitive(msg));
            }
            anyhow::bail!("provider request failed with status {status}");
        }

        let mut accumulated_content = String::new();
        let mut line_buffer = String::new();
        let mut emitted_thinking = false;
        let model = request.model.clone();

        while let Some(chunk) = response.chunk().await.map_err(|e| {
            anyhow::anyhow!("failed to read provider stream chunk: {}", e.without_url())
        })? {
            if accumulated_content.len() > crate::ai::provider::MAX_PROVIDER_RESPONSE_BYTES {
                anyhow::bail!("provider response exceeded 2 MB");
            }
            let chunk_str = String::from_utf8_lossy(&chunk);
            line_buffer.push_str(&chunk_str);

            while let Some(pos) = line_buffer.find('\n') {
                let line = line_buffer[..pos].trim_end_matches('\r').to_string();
                line_buffer.drain(..=pos);

                match parse_sse_line(&line)? {
                    SseLine::Done => break,
                    SseLine::Delta { content, reasoning } => {
                        if reasoning.is_some() && !emitted_thinking {
                            emitted_thinking = true;
                            on_event(StreamEvent::Thinking);
                        }
                        if let Some(text) = content {
                            accumulated_content.push_str(&text);
                            on_event(StreamEvent::Content(&text));
                        }
                    }
                    SseLine::Empty => {}
                }
            }
        }

        if !line_buffer.trim().is_empty() {
            let line = line_buffer.trim_end_matches(['\r', '\n']).to_string();
            match parse_sse_line(&line)? {
                SseLine::Done => {}
                SseLine::Delta { content, reasoning } => {
                    if reasoning.is_some() && !emitted_thinking {
                        on_event(StreamEvent::Thinking);
                    }
                    if let Some(text) = content {
                        accumulated_content.push_str(&text);
                        on_event(StreamEvent::Content(&text));
                    }
                }
                SseLine::Empty => {}
            }
        }

        let trimmed_content = accumulated_content.trim();
        if trimmed_content.is_empty() {
            if emitted_thinking {
                anyhow::bail!(
                    "the reasoning model spent its entire token budget on hidden reasoning and returned no answer"
                );
            }
            anyhow::bail!("provider response contained no assistant content");
        }

        Ok(AiResponse {
            content: crate::security::redact_sensitive(trimmed_content),
            model: crate::security::redact_sensitive(&model),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_formats_chat_completions() {
        let provider = OpenAiCompatProvider::new("https://api.example.com/v1/".to_owned(), None);
        assert_eq!(
            provider.endpoint(),
            "https://api.example.com/v1/chat/completions"
        );
    }
}
