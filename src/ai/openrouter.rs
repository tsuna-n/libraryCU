use anyhow::Context;
use reqwest::Client;

use super::provider::{AiProvider, AiRequest, AiResponse, build_chat_body, parse_chat_response};

const OPENROUTER_ENDPOINT: &str = "https://openrouter.ai/api/v1/chat/completions";

pub struct OpenRouterProvider {
    api_key: String,
    http: Client,
}

impl OpenRouterProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            http: Client::new(),
        }
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
            .bearer_auth(&self.api_key)
            .header("X-Title", "LibraryCU")
            .json(&build_chat_body(&request))
            .send()
            .await
            .context("failed to reach the OpenRouter API")?;
        let status = response.status();
        let payload = response
            .text()
            .await
            .context("failed to read the OpenRouter response")?;
        if !status.is_success() {
            anyhow::bail!("OpenRouter request failed with status {status}");
        }
        parse_chat_response(&payload, &request.model)
    }
}
