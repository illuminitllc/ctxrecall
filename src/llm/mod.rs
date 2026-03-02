pub mod claude_api;
pub mod ollama;
pub mod openai;
pub mod summarizer;

use std::sync::Arc;

use async_trait::async_trait;
use rusqlite::Connection;

use crate::db;
use crate::errors::AppError;

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn summarize(&self, transcript: &str, context: &str) -> Result<String, AppError>;
    fn name(&self) -> &str;
}

/// Creates an LLM provider from database config.
/// Returns `None` if `llm_provider` is not set (summarization silently disabled).
pub fn create_provider(conn: &Connection) -> Option<Arc<dyn LlmProvider>> {
    let provider_name = db::config_repo::get_config(conn, "llm_provider").ok()??;
    let api_key = db::config_repo::get_config(conn, "llm_api_key")
        .ok()
        .flatten();
    let model = db::config_repo::get_config(conn, "llm_model").ok().flatten();

    match provider_name.as_str() {
        "claude" => {
            let key = api_key?;
            Some(Arc::new(claude_api::ClaudeApiProvider::new(key, model)))
        }
        "openai" => {
            let key = api_key?;
            Some(Arc::new(openai::OpenAiProvider::new(key, model)))
        }
        "ollama" => {
            let base_url = db::config_repo::get_config(conn, "llm_ollama_url")
                .ok()
                .flatten();
            Some(Arc::new(ollama::OllamaProvider::new(base_url, model)))
        }
        _ => {
            tracing::warn!("Unknown LLM provider: {provider_name}");
            None
        }
    }
}
