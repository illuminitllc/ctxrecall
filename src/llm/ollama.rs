use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

use super::LlmProvider;
use crate::errors::AppError;

pub struct OllamaProvider {
    client: Client,
    base_url: String,
    model: String,
}

#[derive(Deserialize)]
struct OllamaResponse {
    message: OllamaMessage,
}

#[derive(Deserialize)]
struct OllamaMessage {
    content: String,
}

impl OllamaProvider {
    pub fn new(base_url: Option<String>, model: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.unwrap_or_else(|| "http://localhost:11434".to_string()),
            model: model.unwrap_or_else(|| "llama3.2".to_string()),
        }
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn summarize(&self, transcript: &str, context: &str) -> Result<String, AppError> {
        let body = json!({
            "model": self.model,
            "messages": [
                {
                    "role": "system",
                    "content": "You summarize Claude Code transcripts for issue tracking. \
                                Be concise and focus on decisions, code changes, and next steps."
                },
                {
                    "role": "user",
                    "content": format!("Context: {context}\n\nTranscript:\n{transcript}")
                }
            ],
            "stream": false
        });

        let resp = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::Api(format!("Ollama error: {e}")))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::Api(format!("Ollama: {text}")));
        }

        let result: OllamaResponse = resp
            .json()
            .await
            .map_err(|e| AppError::Api(format!("Parse error: {e}")))?;

        Ok(result.message.content)
    }

    fn name(&self) -> &str {
        "Ollama"
    }
}
