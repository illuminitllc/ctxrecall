use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub id: String,
    pub identifier: String, // e.g. "ENG-123"
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub status_id: Option<String>, // Linear workflow state ID
    pub priority: i32,
    pub assignee: Option<String>,
    pub assignee_id: Option<String>,
    pub team: Option<String>,
    pub team_id: Option<String>,
    pub project: Option<String>,
    pub project_id: Option<String>,
    pub labels: Vec<String>,
    pub url: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    pub id: String,
    pub name: String,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub team_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub id: String,
    pub name: String,
    pub color: String,
    pub team_id: Option<String>, // None = workspace-wide
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueStatus {
    pub id: String,
    pub name: String,
    pub team_id: String,
    pub color: String,
    pub position: f64,
}

#[derive(Debug, Clone)]
pub struct ClaudeSession {
    pub id: Uuid,
    pub issue_id: String,
    pub session_id: String, // Claude CLI session ID
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default)]
pub struct IssueUpdate {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status_id: Option<String>,
    pub assignee_id: Option<String>,
    pub priority: Option<i32>,
    pub label_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct NewIssue {
    pub title: String,
    pub description: Option<String>,
    pub team_id: String,
    pub project_id: Option<String>,
    pub priority: Option<i32>,
    pub assignee_id: Option<String>,
    pub label_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default)]
pub struct IssueFilter {
    pub team_id: Option<String>,
    pub project_id: Option<String>,
    pub status: Option<String>,
    pub assignee: Option<String>,
}
