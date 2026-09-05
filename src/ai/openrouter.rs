use anyhow::Context;
use reqwest::Client;

use super::provider::{
    AiProvider, AiRequest, AiResponse, build_chat_body, parse_chat_response, read_bounded_response,
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
}
