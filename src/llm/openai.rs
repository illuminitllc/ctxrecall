use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

use super::LlmProvider;
use crate::errors::AppError;

pub struct OpenAiProvider {
    client: Client,
    api_key: String,
    model: String,
}

#[derive(Deserialize)]
struct OpenAiResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Deserialize)]
struct Message {
    content: String,
}

impl OpenAiProvider {
    pub fn new(api_key: String, model: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: model.unwrap_or_else(|| "gpt-4o".to_string()),
        }
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
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
            "max_tokens": 2048
        });

        let resp = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::Api(format!("OpenAI error: {e}")))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::Api(format!("OpenAI: {text}")));
        }

        let result: OpenAiResponse = resp
            .json()
            .await
            .map_err(|e| AppError::Api(format!("Parse error: {e}")))?;

        Ok(result
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .unwrap_or_default())
    }

    fn name(&self) -> &str {
        "OpenAI"
    }
}
