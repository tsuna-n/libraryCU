use anyhow::Context;
use reqwest::Client;

use super::provider::{
    AiProvider, AiRequest, AiResponse, StreamEvent, build_chat_body, build_chat_body_streaming,
    parse_chat_response, read_bounded_response,
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
            .timeout(std::time::Duration::from_secs(45))
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
            .timeout(std::time::Duration::from_secs(45));
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
            .timeout(std::time::Duration::from_secs(45));
        if let Some(api_key) = &self.api_key {
            http = http.bearer_auth(api_key);
        }
        let response = http
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

        super::provider::read_chat_stream(response, &request.model, on_event).await
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
