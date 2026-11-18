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

#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModelItem>,
}

#[derive(Debug, Deserialize)]
struct OllamaModelItem {
    name: String,
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

    async fn detect_best_model(&self) -> String {
        let tags_url = format!("{}/api/tags", self.config.base_url.trim_end_matches('/'));
        if let Ok(resp) = self.client.get(&tags_url).timeout(Duration::from_millis(800)).send().await {
            if let Ok(tags) = resp.json::<OllamaTagsResponse>().await {
                let model_names: Vec<String> = tags.models.into_iter().map(|m| m.name.to_lowercase()).collect();
                // Priority preference: 7b/8b models if present, else fallback to 1.5b
                for candidate in ["qwen2.5:7b", "llama3.1:8b", "mistral:latest", "qwen2.5-coder:7b", "qwen2.5:3b"] {
                    if model_names.iter().any(|m| m.contains(candidate)) {
                        return candidate.to_string();
                    }
                }
            }
        }
        self.config.model.clone()
    }

    pub async fn chat(&self, messages: &[ChatMessagePayload]) -> Result<String> {
        let url = format!("{}/api/chat", self.config.base_url.trim_end_matches('/'));
        let selected_model = self.detect_best_model().await;

        let keep_alive = format!("{}s", self.config.unload_after_secs);

        let req = OllamaChatRequest {
            model: selected_model,
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
