use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

use chrono::Utc;
use tokio::sync::mpsc;

use crate::action::Action;
use crate::errors::AppError;

/// Number of trailing lines to keep as an anchor for diffing.
/// Must be large enough to survive minor redraws but small enough to be fast.
const ANCHOR_LINES: usize = 20;

struct TranscriptCapture {
    claude_pane: String,
    transcript_dir: PathBuf,
    issue_id: Option<String>,
    /// Last few lines of the previous capture, used to find where new content starts.
    anchor: Vec<String>,
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
            anchor: Vec::new(),
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
        self.anchor.clear();

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
        self.anchor.clear();
    }

    fn capture(&mut self) -> Result<Option<String>, AppError> {
        if self.claude_pane.is_empty() || self.issue_id.is_none() {
            return Ok(None);
        }

        // Bump the pane's history limit so long sessions aren't truncated.
        // This is idempotent and cheap to run each capture cycle.
        let _ = Command::new("tmux")
            .args([
                "set-option",
                "-t",
                &self.claude_pane,
                "history-limit",
                "50000",
            ])
            .output();

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
        let lines: Vec<&str> = content.lines().collect();

        if lines.is_empty() {
            return Ok(None);
        }

        // Find new content by locating where our anchor appears in the new capture
        let new_content = if self.anchor.is_empty() {
            // First capture — everything is new
            content.clone()
        } else {
            // Try to find the anchor sequence in the new lines.
            // Search backwards from the end for efficiency since anchor should
            // appear somewhere in the middle-to-end of the new capture.
            let anchor_len = self.anchor.len();
            let mut match_pos = None;

            if lines.len() >= anchor_len {
                'outer: for start in (0..=lines.len() - anchor_len).rev() {
                    for (j, anchor_line) in self.anchor.iter().enumerate() {
                        if lines[start + j] != anchor_line.as_str() {
                            continue 'outer;
                        }
                    }
                    match_pos = Some(start + anchor_len);
                    break;
                }
            }

            match match_pos {
                Some(pos) if pos < lines.len() => {
                    // Found anchor — new content is everything after it
                    lines[pos..].join("\n")
                }
                Some(_) => {
                    // Anchor found at the very end — no new content
                    return Ok(None);
                }
                None => {
                    // Anchor not found — scrollback overflowed past our anchor.
                    // Capture everything currently visible; this is the best we can do.
                    tracing::debug!("Transcript anchor lost, capturing full pane");
                    content.clone()
                }
            }
        };

        // Update anchor to the last N lines of the current capture
        let anchor_start = lines.len().saturating_sub(ANCHOR_LINES);
        self.anchor = lines[anchor_start..].iter().map(|s| s.to_string()).collect();

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
            // First tick fires immediately
            ticker.tick().await;
            loop {
                // Capture first, then wait — ensures we capture right away on set_issue
                {
                    let mut guard = poll_inner.lock().unwrap();
                    if guard.stopped {
                        break;
                    }
                    match guard.capture() {
                        Ok(Some(_)) => {}
                        Ok(None) => {}
                        Err(e) => {
                            tracing::warn!("Transcript capture error: {e}");
                        }
                    }
                }
                ticker.tick().await;
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
