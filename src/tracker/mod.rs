pub mod linear;
pub mod sync;
pub mod types;

use async_trait::async_trait;

use crate::errors::AppError;
use types::{Issue, IssueFilter, IssueStatus, IssueUpdate, Label, NewIssue, Project, Team};

#[async_trait]
pub trait IssueTracker: Send + Sync {
    async fn list_issues(&self, filter: &IssueFilter) -> Result<Vec<Issue>, AppError>;
    async fn get_issue(&self, id: &str) -> Result<Issue, AppError>;
    async fn update_issue(&self, id: &str, update: &IssueUpdate) -> Result<Issue, AppError>;
    async fn create_issue(&self, new_issue: &NewIssue) -> Result<Issue, AppError>;
    async fn list_teams(&self) -> Result<Vec<Team>, AppError>;
    async fn list_projects(&self) -> Result<Vec<Project>, AppError>;
    async fn list_workflow_states(&self) -> Result<Vec<IssueStatus>, AppError>;
    async fn list_labels(&self) -> Result<Vec<Label>, AppError>;
}
