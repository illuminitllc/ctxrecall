pub mod layout;

use std::env;
use std::process::Command;

use crate::errors::AppError;

pub struct TmuxManager {
    pub session_name: String,
    pub tui_pane: String,
    pub claude_pane: String,
}

impl TmuxManager {
    pub fn is_inside_tmux() -> bool {
        env::var("TMUX").is_ok()
    }

    pub fn bootstrap() -> Result<(), AppError> {
        let exe = env::current_exe()?;
        let exe_str = exe.to_string_lossy();
        let session = "ctxrecall";

        // Create a new tmux session running the TUI directly (single pane).
        // The Claude pane is created on-demand when the user first launches a session.
        let status = Command::new("tmux")
            .args([
                "new-session",
                "-d",
                "-s",
                session,
                "-x",
                "200",
                "-y",
                "50",
                &format!("{exe_str} --in-tmux"),
            ])
            .status()?;

        if !status.success() {
            return Err(AppError::Tmux("Failed to create tmux session".into()));
        }

        // Attach to the session
        let status = Command::new("tmux")
            .args(["attach-session", "-t", session])
            .status()?;

        if !status.success() {
            return Err(AppError::Tmux("Failed to attach to tmux session".into()));
        }

        Ok(())
    }

    pub fn new() -> Result<Self, AppError> {
        let session_name =
            env::var("TMUX").map_err(|_| AppError::Tmux("Not inside tmux".into()))?;

        // Get current pane ID (this is the TUI pane)
        let output = Command::new("tmux")
            .args(["display-message", "-p", "#{pane_id}"])
            .output()?;

        let tui_pane = String::from_utf8_lossy(&output.stdout).trim().to_string();

        // Check if a second pane already exists (e.g. from a previous run)
        let output = Command::new("tmux")
            .args(["list-panes", "-F", "#{pane_id}"])
            .output()?;

        let panes: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();

        let claude_pane = panes
            .into_iter()
            .find(|p| p != &tui_pane)
            .unwrap_or_default();

        Ok(Self {
            session_name,
            tui_pane,
            claude_pane,
        })
    }

    /// Create the Claude pane by splitting the window. Returns the new pane ID.
    /// The TUI pane shrinks to the given percentage of the window width.
    pub fn create_claude_pane(&mut self, tui_percent: u16) -> Result<(), AppError> {
        if !self.claude_pane.is_empty() {
            return Ok(()); // Already exists
        }

        // Split left of the TUI pane — the new pane becomes the Claude pane
        let claude_percent = 100u16.saturating_sub(tui_percent);
        let output = Command::new("tmux")
            .args([
                "split-window",
                "-h",
                "-b",          // insert before (left of) current pane
                "-t",
                &self.tui_pane,
                "-p",
                &claude_percent.to_string(),
                "-P",          // print pane info
                "-F",
                "#{pane_id}",
            ])
            .output()?;

        if !output.status.success() {
            return Err(AppError::Tmux("Failed to create Claude pane".into()));
        }

        self.claude_pane = String::from_utf8_lossy(&output.stdout).trim().to_string();
        tracing::info!("Created Claude pane: {}", self.claude_pane);

        // Focus back to the TUI pane (split-window focuses the new pane)
        let _ = Command::new("tmux")
            .args(["select-pane", "-t", &self.tui_pane])
            .status();

        Ok(())
    }

    pub fn send_keys_to_claude(&self, keys: &str) -> Result<(), AppError> {
        if self.claude_pane.is_empty() {
            return Err(AppError::Tmux("No Claude pane found".into()));
        }
        Command::new("tmux")
            .args(["send-keys", "-t", &self.claude_pane, keys, "Enter"])
            .status()?;
        Ok(())
    }

    /// Switch tmux focus to the Claude pane.
    pub fn focus_claude_pane(&self) -> Result<(), AppError> {
        if self.claude_pane.is_empty() {
            return Err(AppError::Tmux("No Claude pane found".into()));
        }
        let status = Command::new("tmux")
            .args(["select-pane", "-t", &self.claude_pane])
            .status()?;
        if !status.success() {
            return Err(AppError::Tmux("Failed to focus Claude pane".into()));
        }
        Ok(())
    }

    /// Resize the TUI pane to a given percentage of the window width.
    pub fn resize_tui_pane(&self, percent: u16) -> Result<(), AppError> {
        if self.tui_pane.is_empty() {
            return Err(AppError::Tmux("No TUI pane found".into()));
        }
        let status = Command::new("tmux")
            .args([
                "resize-pane",
                "-t",
                &self.tui_pane,
                "-x",
                &format!("{}%", percent),
            ])
            .status()?;
        if !status.success() {
            return Err(AppError::Tmux("Failed to resize TUI pane".into()));
        }
        Ok(())
    }
}
