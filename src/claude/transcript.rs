use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

use chrono::Utc;
use tokio::sync::mpsc;

use crate::action::Action;
use crate::errors::AppError;

struct TranscriptCapture {
    claude_pane: String,
    transcript_dir: PathBuf,
    issue_id: Option<String>,
    last_content: String,
    current_file: Option<PathBuf>,
    action_tx: mpsc::UnboundedSender<Action>,
    stopped: bool,
}

impl TranscriptCapture {
    fn new(
        claude_pane: String,
        transcript_dir: PathBuf,
        action_tx: mpsc::UnboundedSender<Action>,
    ) -> Self {
        Self {
            claude_pane,
            transcript_dir,
            issue_id: None,
            last_content: String::new(),
            current_file: None,
            action_tx,
            stopped: false,
        }
    }

    fn set_issue(&mut self, issue_id: &str) {
        if self.issue_id.as_deref() == Some(issue_id) {
            return;
        }
        self.issue_id = Some(issue_id.to_string());
        self.last_content.clear();

        // Create new transcript file
        let dir = self.transcript_dir.join(issue_id);
        std::fs::create_dir_all(&dir).ok();
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let file = dir.join(format!("{timestamp}.txt"));
        self.current_file = Some(file);
    }

    fn clear_issue(&mut self) {
        self.issue_id = None;
        self.current_file = None;
        self.last_content.clear();
    }

    fn capture(&mut self) -> Result<Option<String>, AppError> {
        if self.claude_pane.is_empty() || self.issue_id.is_none() {
            return Ok(None);
        }

        let output = Command::new("tmux")
            .args([
                "capture-pane",
                "-t",
                &self.claude_pane,
                "-p",
                "-S",
                "-",
            ])
            .output()?;

        if !output.status.success() {
            return Ok(None);
        }

        let content = String::from_utf8_lossy(&output.stdout).to_string();

        if content == self.last_content {
            return Ok(None);
        }

        // Find new content (diff)
        let new_content = if self.last_content.is_empty() {
            content.clone()
        } else {
            // Simple diff: find lines after the last known content
            let old_lines: Vec<&str> = self.last_content.lines().collect();
            let new_lines: Vec<&str> = content.lines().collect();

            if new_lines.len() > old_lines.len() {
                new_lines[old_lines.len()..].join("\n")
            } else {
                // Content changed entirely (scrolled), capture full
                content.clone()
            }
        };

        self.last_content = content;

        // Append to file
        if let Some(file) = &self.current_file {
            if !new_content.trim().is_empty() {
                use std::io::Write;
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(file)
                {
                    let _ = writeln!(f, "{new_content}");
                }
            }
        }

        if new_content.trim().is_empty() {
            Ok(None)
        } else {
            Ok(Some(new_content))
        }
    }
}

/// Thread-safe handle for transcript capture that supports multi-session reuse.
/// The polling loop runs in a background task; callers use `set_issue()` / `clear_issue()`
/// to control which issue is being captured.
pub struct TranscriptCaptureHandle {
    inner: Arc<Mutex<TranscriptCapture>>,
}

impl TranscriptCaptureHandle {
    /// Creates a new capture handle and starts the background polling task immediately.
    pub fn new(
        claude_pane: String,
        transcript_dir: PathBuf,
        action_tx: mpsc::UnboundedSender<Action>,
        interval_secs: u64,
    ) -> Self {
        let capture = TranscriptCapture::new(claude_pane, transcript_dir, action_tx);
        let inner = Arc::new(Mutex::new(capture));

        // Start background polling task
        let poll_inner = inner.clone();
        tokio::spawn(async move {
            let mut ticker =
                tokio::time::interval(std::time::Duration::from_secs(interval_secs));
            loop {
                ticker.tick().await;
                let mut guard = poll_inner.lock().unwrap();
                if guard.stopped {
                    break;
                }
                match guard.capture() {
                    Ok(Some(_)) => {} // New transcript content captured
                    Ok(None) => {}
                    Err(e) => {
                        tracing::warn!("Transcript capture error: {e}");
                    }
                }
            }
        });

        Self { inner }
    }

    /// Set the active issue — creates a new transcript file and starts capturing.
    pub fn set_issue(&self, issue_id: &str) {
        self.inner.lock().unwrap().set_issue(issue_id);
    }

    /// Clear the active issue — pauses capture (polling loop no-ops).
    pub fn clear_issue(&self) {
        self.inner.lock().unwrap().clear_issue();
    }

    /// Signal the polling loop to exit.
    pub fn stop(&self) {
        self.inner.lock().unwrap().stopped = true;
    }
}

impl Drop for TranscriptCaptureHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

pub fn transcript_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("transcripts")
}
