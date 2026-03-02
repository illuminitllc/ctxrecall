use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

use super::LlmProvider;
use crate::errors::AppError;

pub struct ClaudeApiProvider {
    client: Client,
    api_key: String,
    model: String,
}

#[derive(Deserialize)]
struct ClaudeResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    text: Option<String>,
}

impl ClaudeApiProvider {
    pub fn new(api_key: String, model: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: model.unwrap_or_else(|| "claude-haiku-4-5-20251001".to_string()),
        }
    }
}

#[async_trait]
impl LlmProvider for ClaudeApiProvider {
    async fn summarize(&self, transcript: &str, context: &str) -> Result<String, AppError> {
        let body = json!({
            "model": self.model,
            "max_tokens": 2048,
            "messages": [{
                "role": "user",
                "content": format!(
                    "Summarize the following Claude Code transcript for issue tracking purposes.\n\
                     Context: {context}\n\n\
                     Transcript:\n{transcript}\n\n\
                     Provide a concise summary covering: what was discussed, decisions made, \
                     code changes, and next steps."
                )
            }]
        });

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::Api(format!("Claude API error: {e}")))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::Api(format!("Claude API: {text}")));
        }

        let result: ClaudeResponse = resp
            .json()
            .await
            .map_err(|e| AppError::Api(format!("Parse error: {e}")))?;

        Ok(result
            .content
            .into_iter()
            .filter_map(|b| b.text)
            .collect::<Vec<_>>()
            .join("\n"))
    }

    fn name(&self) -> &str {
        "Claude API"
    }
}
