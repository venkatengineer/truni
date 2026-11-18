use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use truni_core::config::AiConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessagePayload {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<ChatMessagePayload>,
    stream: bool,
    options: Option<OllamaOptions>,
    keep_alive: Option<String>,
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_ctx: usize,
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: ChatMessagePayload,
}

pub struct LlmClient {
    client: Client,
    config: AiConfig,
}

impl LlmClient {
    pub fn new(config: AiConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_default();

        Self { client, config }
    }

    pub async fn chat(&self, messages: &[ChatMessagePayload]) -> Result<String> {
        let url = format!("{}/api/chat", self.config.base_url.trim_end_matches('/'));

        let keep_alive = format!("{}s", self.config.unload_after_secs);

        let req = OllamaChatRequest {
            model: self.config.model.clone(),
            messages: messages.to_vec(),
            stream: false,
            options: Some(OllamaOptions {
                temperature: self.config.temperature,
                num_ctx: self.config.context_limit,
            }),
            keep_alive: Some(keep_alive),
        };

        let resp = self.client.post(&url)
            .json(&req)
            .send()
            .await
            .with_context(|| format!("Failed to contact LLM backend at {}", url))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("LLM request failed with status {}: {}", status, body);
        }

        let chat_res: OllamaChatResponse = resp.json().await
            .context("Failed to deserialize LLM chat response")?;

        Ok(chat_res.message.content)
    }
}
