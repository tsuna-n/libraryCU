//! Explicit doctor probe. No project context or generation request is sent.
use crate::config::AiConfig;

pub fn check(ai: &AiConfig, timeout: u64) -> anyhow::Result<()> {
    let key = super::credential(ai, |name| std::env::var(name).ok())?;
    anyhow::ensure!(ai.provider != "off", "AI is off");
    let base = if ai.provider == "openrouter" {
        "https://openrouter.ai/api/v1"
    } else {
        ai.effective_base_url()
    };
    let mut url = reqwest::Url::parse(&format!("{}/models", base.trim_end_matches('/')))
        .map_err(|_| anyhow::anyhow!("invalid provider base URL"))?;
    anyhow::ensure!(
        matches!(url.scheme(), "http" | "https") && url.host_str().is_some(),
        "provider URL must use HTTP or HTTPS"
    );
    // Configured URI credentials must never become accidental authentication.
    anyhow::ensure!(
        url.username().is_empty() && url.password().is_none(),
        "provider URL must not contain credentials"
    );
    url.set_fragment(None);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(timeout))
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        let mut request = http.get(url);
        if let Some(key) = key {
            request = request.bearer_auth(key);
        }
        let response = request.send().await.map_err(|error| {
            anyhow::anyhow!(if error.is_timeout() {
                "connectivity check timed out"
            } else {
                "provider models endpoint unavailable"
            })
        })?;
        let (status, body) = super::provider::read_bounded_response(response).await?;
        anyhow::ensure!(
            status.is_success(),
            "models endpoint returned HTTP {}",
            status.as_u16()
        );
        let value: serde_json::Value = serde_json::from_str(&body)
            .map_err(|_| anyhow::anyhow!("models endpoint returned invalid JSON"))?;
        anyhow::ensure!(
            value.get("data").is_some_and(serde_json::Value::is_array),
            "models endpoint returned no model list"
        );
        Ok(())
    })
}
