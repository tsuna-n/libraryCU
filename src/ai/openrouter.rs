use anyhow::Context;
use reqwest::Client;

use super::provider::{
    AiProvider, AiRequest, AiResponse, SseLine, StreamEvent, build_chat_body,
    build_chat_body_streaming, parse_chat_response, parse_sse_line, read_bounded_response,
};

const OPENROUTER_ENDPOINT: &str = "https://openrouter.ai/api/v1/chat/completions";

pub struct OpenRouterProvider {
    api_key: String,
    http: Client,
}

impl OpenRouterProvider {
    pub fn new(api_key: String) -> Self {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(45))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self { api_key, http }
    }
}

impl AiProvider for OpenRouterProvider {
    fn name(&self) -> &'static str {
        "openrouter"
    }

    async fn chat(&self, request: AiRequest) -> anyhow::Result<AiResponse> {
        let response = self
            .http
            .post(OPENROUTER_ENDPOINT)
            .timeout(std::time::Duration::from_secs(45))
            .bearer_auth(&self.api_key)
            .header("X-Title", "libraryCube")
            .json(&build_chat_body(&request))
            .send()
            .await
            .map_err(reqwest::Error::without_url)
            .context("failed to reach the OpenRouter API")?;
        let (status, payload) = read_bounded_response(response).await?;
        if !status.is_success() {
            anyhow::bail!("OpenRouter request failed with status {status}");
        }
        parse_chat_response(&payload, &request.model)
    }

    async fn chat_stream<'a>(
        &'a self,
        request: AiRequest,
        on_event: &'a mut (dyn FnMut(StreamEvent) + Send),
    ) -> anyhow::Result<AiResponse> {
        let mut response = self
            .http
            .post(OPENROUTER_ENDPOINT)
            .timeout(std::time::Duration::from_secs(45))
            .bearer_auth(&self.api_key)
            .header("X-Title", "libraryCube")
            .json(&build_chat_body_streaming(&request, true))
            .send()
            .await
            .map_err(reqwest::Error::without_url)
            .context("failed to reach the OpenRouter API")?;
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
            anyhow::bail!("OpenRouter request failed with status {status}");
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
