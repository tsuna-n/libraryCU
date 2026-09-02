pub mod context;
pub mod enhance;
pub mod openai_compat;
pub mod openrouter;
pub mod provider;

pub use context::{MAX_ERROR_CONTEXT_CHARS, build_request};
pub use enhance::{apply_response, enhance, enhance_with_client};
pub use openai_compat::OpenAiCompatProvider;
pub use openrouter::OpenRouterProvider;
pub use provider::{
    AiClient, AiProvider, AiRequest, AiResponse, CONFIDENCE_MARKER, parse_confidence,
    strip_confidence_marker,
};

use serde::Serialize;

use crate::config::settings::AiConfig;

#[derive(Debug, Clone, Serialize)]
pub struct AiContribution {
    pub provider: String,
    pub model: String,
    pub analysis: String,
    pub confidence: String,
}

/// Resolve the configured provider into a client, or explain why AI is unavailable.
pub fn resolve_client(ai: &AiConfig) -> anyhow::Result<AiClient> {
    match ai.provider.as_str() {
        "openrouter" => {
            let api_key = std::env::var("OPENROUTER_API_KEY")
                .map_err(|_| anyhow::anyhow!("OPENROUTER_API_KEY is not set"))?;
            Ok(AiClient::OpenRouter(OpenRouterProvider::new(api_key)))
        }
        "openai-compat" => {
            let api_key = std::env::var("OPENAI_API_KEY").ok();
            Ok(AiClient::OpenAiCompat(OpenAiCompatProvider::new(
                ai.base_url.clone(),
                api_key,
            )))
        }
        other => anyhow::bail!(
            "unsupported AI provider {other:?}; supported providers: openrouter, openai-compat"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openrouter_requires_an_api_key() {
        let ai = AiConfig {
            provider: "openrouter".to_owned(),
            model: "test-model".to_owned(),
            base_url: String::new(),
        };
        unsafe { std::env::remove_var("OPENROUTER_API_KEY") };
        assert!(resolve_client(&ai).is_err());
    }

    #[test]
    fn openai_compat_uses_the_configured_base_url_without_a_key() {
        let ai = AiConfig {
            provider: "openai-compat".to_owned(),
            model: "test-model".to_owned(),
            base_url: "http://127.0.0.1:11434/v1".to_owned(),
        };
        unsafe { std::env::remove_var("OPENAI_API_KEY") };
        let client = resolve_client(&ai).expect("openai-compat should not require a key");
        match client {
            AiClient::OpenAiCompat(provider) => {
                assert_eq!(
                    provider.endpoint(),
                    "http://127.0.0.1:11434/v1/chat/completions"
                );
            }
            _ => panic!("expected the OpenAI-compatible client"),
        }
    }
}
