use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;

use super::LlmProvider;
use crate::action::Action;

pub struct Summarizer {
    provider: Arc<dyn LlmProvider>,
    transcript_dir: PathBuf,
    action_tx: mpsc::UnboundedSender<Action>,
}

impl Summarizer {
    pub fn new(
        provider: Arc<dyn LlmProvider>,
        transcript_dir: PathBuf,
        action_tx: mpsc::UnboundedSender<Action>,
    ) -> Self {
        Self {
            provider,
            transcript_dir,
            action_tx,
        }
    }

    pub fn start(self, interval: Duration) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                if let Err(e) = self.summarize_pending().await {
                    tracing::error!("Summarization error: {e}");
                }
            }
        })
    }

    async fn summarize_pending(&self) -> Result<(), crate::errors::AppError> {
        // Find transcript files that don't have corresponding summary files
        let entries = match std::fs::read_dir(&self.transcript_dir) {
            Ok(e) => e,
            Err(_) => return Ok(()), // No transcripts yet
        };

        for issue_dir in entries.flatten() {
            if !issue_dir.path().is_dir() {
                continue;
            }

            let issue_id = issue_dir
                .file_name()
                .to_string_lossy()
                .to_string();

            let transcripts = match std::fs::read_dir(issue_dir.path()) {
                Ok(e) => e,
                Err(_) => continue,
            };

            for file in transcripts.flatten() {
                let path = file.path();
                let name = path.file_name().unwrap_or_default().to_string_lossy();

                // Skip summary files and non-txt files
                if name.ends_with(".summary.md") || !name.ends_with(".txt") {
                    continue;
                }

                let summary_path = path.with_extension("summary.md");
                if summary_path.exists() {
                    continue; // Already summarized
                }

                // Read transcript
                let transcript = match std::fs::read_to_string(&path) {
                    Ok(t) if !t.trim().is_empty() => t,
                    _ => continue,
                };

                // Summarize
                let context = format!("Issue: {issue_id}");
                tracing::info!("Summarizing transcript for {issue_id}: {}", path.display());

                match self.provider.summarize(&transcript, &context).await {
                    Ok(summary) => {
                        std::fs::write(&summary_path, &summary)?;
                        let _ = self.action_tx.send(Action::StatusMessage(format!(
                            "Summary generated for {issue_id}"
                        )));
                        tracing::info!("Summary saved: {}", summary_path.display());
                    }
                    Err(e) => {
                        tracing::error!("Failed to summarize {}: {e}", path.display());
                    }
                }
            }
        }

        Ok(())
    }
}
