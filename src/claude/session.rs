use std::io::Write;
use std::process::Command;
use std::thread;
use std::time::Duration;

use rusqlite::Connection;
use tempfile::NamedTempFile;
use uuid::Uuid;

use crate::db::session_repo;
use crate::errors::AppError;

pub struct ClaudeManager {
    claude_pane: String,
    active_session_db_id: Option<String>,
    active_issue_id: Option<String>,
    active_claude_session_id: Option<String>,
}

impl ClaudeManager {
    pub fn new(claude_pane: String) -> Self {
        Self {
            claude_pane,
            active_session_db_id: None,
            active_issue_id: None,
            active_claude_session_id: None,
        }
    }

    pub fn pane_id(&self) -> &str {
        &self.claude_pane
    }

    pub fn active_issue_id(&self) -> Option<&str> {
        self.active_issue_id.as_deref()
    }

    pub fn active_claude_session_id(&self) -> Option<&str> {
        self.active_claude_session_id.as_deref()
    }

    pub fn launch_for_issue(
        &mut self,
        conn: &Connection,
        issue_id: &str,
        issue_identifier: &str,
        working_dir: Option<&str>,
    ) -> Result<(), AppError> {
        if self.claude_pane.is_empty() {
            return Err(AppError::Tmux("No Claude pane available".into()));
        }

        // If already on this issue, do nothing
        if self.active_issue_id.as_deref() == Some(issue_id) {
            tracing::info!("Claude already active for issue {issue_identifier}");
            return Ok(());
        }

        // Exit current session if any
        if self.active_session_db_id.is_some() {
            self.exit_current_session(conn)?;
        }

        // Change to the working directory if configured
        if let Some(dir) = working_dir {
            tracing::info!("Changing to working directory: {dir}");
            self.tmux_send_keys(&format!("cd {}", shell_escape(dir)))?;
            thread::sleep(Duration::from_millis(100));
        }

        // Always create a fresh session. The CLI has no flag to programmatically resume
        // a specific session by ID (--session-id errors if it exists, --resume opens a picker).
        // All session IDs are tracked in the DB per issue for reference.
        let claude_session_id = Uuid::new_v4().to_string();
        tracing::info!("Starting Claude session {claude_session_id} for {issue_identifier}");
        let cmd = format!("claude --session-id {claude_session_id}");

        // Send to tmux pane
        self.tmux_send_keys(&cmd)?;

        // Store in DB
        let db_id = Uuid::new_v4().to_string();
        session_repo::save_session(conn, &db_id, issue_id, &claude_session_id)?;

        self.active_session_db_id = Some(db_id);
        self.active_issue_id = Some(issue_id.to_string());
        self.active_claude_session_id = Some(claude_session_id);

        Ok(())
    }

    pub fn exit_current_session(&mut self, conn: &Connection) -> Result<(), AppError> {
        if let Some(db_id) = self.active_session_db_id.take() {
            // Kill whatever is running in the pane and respawn a fresh shell.
            // This is far more reliable than sending /exit and waiting for it
            // to be processed — no race conditions, no timing issues.
            self.respawn_pane()?;
            session_repo::end_session(conn, &db_id)?;
            self.active_issue_id = None;
            self.active_claude_session_id = None;
            tracing::info!("Ended Claude session {db_id}");
        }
        Ok(())
    }

    /// Inject context into the Claude pane using tmux's load-buffer/paste-buffer.
    /// Failures are logged as warnings and do not propagate — context injection is best-effort.
    pub fn inject_context(pane: &str, context: &str) -> Result<(), AppError> {
        // Write context to a temp file (auto-cleaned when _tmp drops)
        let mut tmp = NamedTempFile::new()?;
        tmp.write_all(context.as_bytes())?;
        tmp.flush()?;
        let tmp_path = tmp.path().to_string_lossy().to_string();

        // Load file into tmux buffer
        let status = Command::new("tmux")
            .args(["load-buffer", &tmp_path])
            .status();
        match status {
            Ok(s) if s.success() => {}
            Ok(s) => {
                tracing::warn!("tmux load-buffer exited with {s}");
                return Ok(());
            }
            Err(e) => {
                tracing::warn!("tmux load-buffer failed: {e}");
                return Ok(());
            }
        }

        // Paste buffer into pane
        let status = Command::new("tmux")
            .args(["paste-buffer", "-t", pane])
            .status();
        match status {
            Ok(s) if s.success() => {}
            Ok(s) => {
                tracing::warn!("tmux paste-buffer exited with {s}");
                return Ok(());
            }
            Err(e) => {
                tracing::warn!("tmux paste-buffer failed: {e}");
                return Ok(());
            }
        }

        // Send Enter to submit the pasted text
        let status = Command::new("tmux")
            .args(["send-keys", "-t", pane, "", "Enter"])
            .status();
        match status {
            Ok(s) if s.success() => {}
            Ok(s) => {
                tracing::warn!("tmux send-keys exited with {s}");
            }
            Err(e) => {
                tracing::warn!("tmux send-keys failed: {e}");
            }
        }

        Ok(())
    }

    /// Kill the current process in the Claude pane and start a fresh shell.
    /// The pane ID stays the same so all references remain valid.
    fn respawn_pane(&self) -> Result<(), AppError> {
        let status = Command::new("tmux")
            .args(["respawn-pane", "-t", &self.claude_pane, "-k"])
            .status()?;

        if !status.success() {
            return Err(AppError::Tmux(format!(
                "Failed to respawn pane {}",
                self.claude_pane
            )));
        }

        // Brief pause for the new shell to initialize
        thread::sleep(Duration::from_millis(300));
        Ok(())
    }

    fn tmux_send_keys(&self, keys: &str) -> Result<(), AppError> {
        let status = Command::new("tmux")
            .args(["send-keys", "-t", &self.claude_pane, keys, "Enter"])
            .status()?;

        if !status.success() {
            return Err(AppError::Tmux(format!(
                "Failed to send keys to pane {}",
                self.claude_pane
            )));
        }

        Ok(())
    }
}

/// Shell-escape a path for safe use in a command.
fn shell_escape(s: &str) -> String {
    if s.contains(|c: char| c.is_whitespace() || "\"'\\$`!#&|;(){}[]<>?*~".contains(c)) {
        format!("'{}'", s.replace('\'', "'\\''"))
    } else {
        s.to_string()
    }
}
