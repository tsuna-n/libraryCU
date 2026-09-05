use anyhow::Context;
use reqwest::Client;

use super::provider::{
    AiProvider, AiRequest, AiResponse, build_chat_body, parse_chat_response, read_bounded_response,
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
        let mut http = self.http.post(self.endpoint());
        if let Some(api_key) = &self.api_key {
            http = http.bearer_auth(api_key);
        }
        let response = http
            .json(&build_chat_body(&request))
            .send()
            .await
            .context("failed to reach the OpenAI-compatible endpoint")?;
        let (status, payload) = read_bounded_response(response).await?;
        if !status.is_success() {
            anyhow::bail!("provider request failed with status {status}");
        }
        parse_chat_response(&payload, &request.model)
    }
}
