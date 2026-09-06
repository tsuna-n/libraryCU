pub mod context;
pub mod enhance;
pub mod openai_compat;
pub mod openrouter;
pub mod prompt;
pub mod provider;

pub use context::{MAX_ERROR_CONTEXT_CHARS, build_request, build_request_with_language};
pub use enhance::{apply_response, enhance, enhance_with_client, enhance_with_language};
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
    resolve_client_with_env(ai, |name| std::env::var(name).ok())
}

fn resolve_client_with_env(
    ai: &AiConfig,
    env_value: impl Fn(&str) -> Option<String>,
) -> anyhow::Result<AiClient> {
    match ai.provider.as_str() {
        "openrouter" => {
            let api_key = env_value("OPENROUTER_API_KEY")
                .filter(|key| !key.trim().is_empty())
                .ok_or_else(|| anyhow::anyhow!("OPENROUTER_API_KEY is not set or is empty"))?;
            Ok(AiClient::OpenRouter(OpenRouterProvider::new(api_key)))
        }
        "openai" => {
            let api_key = env_value("OPENAI_API_KEY")
                .filter(|key| !key.trim().is_empty())
                .ok_or_else(|| anyhow::anyhow!("OPENAI_API_KEY is not set or is empty"))?;
            Ok(AiClient::OpenAiCompat(OpenAiCompatProvider::new(
                ai.effective_base_url().to_owned(),
                Some(api_key),
            )))
        }
        "zai" | "glm" => {
            let api_key = ["ZAI_API_KEY", "GLM_API_KEY"]
                .iter()
                .filter_map(|name| env_value(name))
                .find(|key| !key.trim().is_empty())
                .ok_or_else(|| anyhow::anyhow!("ZAI_API_KEY (or GLM_API_KEY) is not set or is empty"))?;
            Ok(AiClient::OpenAiCompat(OpenAiCompatProvider::new(
                ai.effective_base_url().to_owned(),
                Some(api_key),
            )))
        }
        "ollama" => {
            let api_key = env_value("OLLAMA_API_KEY").filter(|key| !key.trim().is_empty());
            Ok(AiClient::OpenAiCompat(OpenAiCompatProvider::new(
                ai.effective_base_url().to_owned(),
                api_key,
            )))
        }
        "openai-compat" => {
            // Key is optional: local servers need none. Vendor env vars are
            // accepted for convenience (GLM_API_KEY, ZAI_API_KEY, then OPENAI_API_KEY).
            let api_key = ["GLM_API_KEY", "ZAI_API_KEY", "OPENAI_API_KEY"]
                .iter()
                .filter_map(|name| env_value(name))
                .find(|key| !key.trim().is_empty());
            Ok(AiClient::OpenAiCompat(OpenAiCompatProvider::new(
                ai.effective_base_url().to_owned(),
                api_key,
            )))
        }
        other => anyhow::bail!(
            "unsupported AI provider {other:?}; supported providers: openai, zai (glm), ollama, openrouter, openai-compat"
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
        assert!(resolve_client_with_env(&ai, |_| None).is_err());
        assert!(resolve_client_with_env(&ai, |_| Some(" ".into())).is_err());
    }

    #[test]
    fn openai_requires_an_api_key_and_uses_default_endpoint() {
        let ai = AiConfig {
            provider: "openai".to_owned(),
            model: String::new(),
            base_url: String::new(),
        };
        assert!(resolve_client_with_env(&ai, |_| None).is_err());
        let client = resolve_client_with_env(&ai, |name| {
            if name == "OPENAI_API_KEY" {
                Some("sk-test".into())
            } else {
                None
            }
        })
        .expect("openai should succeed with OPENAI_API_KEY");
        match client {
            AiClient::OpenAiCompat(provider) => {
                assert_eq!(
                    provider.endpoint(),
                    "https://api.openai.com/v1/chat/completions"
                );
            }
            _ => panic!("expected OpenAiCompat client"),
        }
    }

    #[test]
    fn zai_accepts_either_zai_or_glm_key() {
        let ai = AiConfig {
            provider: "zai".to_owned(),
            model: String::new(),
            base_url: String::new(),
        };
        assert!(resolve_client_with_env(&ai, |_| None).is_err());
        let client = resolve_client_with_env(&ai, |name| {
            if name == "ZAI_API_KEY" {
                Some("test-zai-key".into())
            } else {
                None
            }
        })
        .expect("zai should succeed with ZAI_API_KEY");
        match client {
            AiClient::OpenAiCompat(provider) => {
                assert_eq!(
                    provider.endpoint(),
                    "https://open.bigmodel.cn/api/paas/v4/chat/completions"
                );
            }
            _ => panic!("expected OpenAiCompat client"),
        }

        let glm_ai = AiConfig {
            provider: "glm".to_owned(),
            model: String::new(),
            base_url: String::new(),
        };
        let client_glm = resolve_client_with_env(&glm_ai, |name| {
            if name == "GLM_API_KEY" {
                Some("test-glm-key".into())
            } else {
                None
            }
        })
        .expect("glm should succeed with GLM_API_KEY");
        match client_glm {
            AiClient::OpenAiCompat(provider) => {
                assert_eq!(
                    provider.endpoint(),
                    "https://open.bigmodel.cn/api/paas/v4/chat/completions"
                );
            }
            _ => panic!("expected OpenAiCompat client"),
        }
    }

    #[test]
    fn ollama_resolves_without_api_key() {
        let ai = AiConfig {
            provider: "ollama".to_owned(),
            model: String::new(),
            base_url: String::new(),
        };
        let client = resolve_client_with_env(&ai, |_| None).expect("ollama needs no key");
        match client {
            AiClient::OpenAiCompat(provider) => {
                assert_eq!(
                    provider.endpoint(),
                    "http://localhost:11434/v1/chat/completions"
                );
            }
            _ => panic!("expected OpenAiCompat client"),
        }
    }

    #[test]
    fn openai_compat_uses_the_configured_base_url_without_a_key() {
        let ai = AiConfig {
            provider: "openai-compat".to_owned(),
            model: "test-model".to_owned(),
            base_url: "http://127.0.0.1:11434/v1".to_owned(),
        };
        let client =
            resolve_client_with_env(&ai, |_| None).expect("openai-compat should not require a key");
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
