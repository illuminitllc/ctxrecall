use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;

use super::IssueTracker;
use super::types::IssueFilter;
use crate::action::Action;

pub struct SyncManager {
    tracker: Arc<dyn IssueTracker>,
    action_tx: mpsc::UnboundedSender<Action>,
}

impl SyncManager {
    pub fn new(
        tracker: Arc<dyn IssueTracker>,
        action_tx: mpsc::UnboundedSender<Action>,
    ) -> Self {
        Self { tracker, action_tx }
    }

    pub fn start_background_sync(self, interval: Duration) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            // Initial fetch immediately
            self.fetch_all().await;

            let mut ticker = tokio::time::interval(interval);
            ticker.tick().await; // consume immediate first tick

            loop {
                ticker.tick().await;
                self.fetch_all().await;
            }
        })
    }

    async fn fetch_all(&self) {
        self.fetch_issues().await;
        self.fetch_workflow_states().await;
        self.fetch_teams().await;
        self.fetch_projects().await;
        self.fetch_labels().await;
    }

    async fn fetch_issues(&self) {
        let filter = IssueFilter::default();
        match self.tracker.list_issues(&filter).await {
            Ok(issues) => {
                let count = issues.len();
                let _ = self.action_tx.send(Action::IssuesLoaded(issues));
                let _ = self
                    .action_tx
                    .send(Action::StatusMessage(format!("Loaded {count} issues")));
                tracing::info!("Synced {count} issues from Linear");
            }
            Err(e) => {
                tracing::error!("Failed to sync issues: {e}");
                let _ = self
                    .action_tx
                    .send(Action::Error(format!("Sync failed: {e}")));
            }
        }
    }

    async fn fetch_workflow_states(&self) {
        match self.tracker.list_workflow_states().await {
            Ok(states) => {
                let count = states.len();
                let _ = self.action_tx.send(Action::WorkflowStatesLoaded(states));
                tracing::info!("Synced {count} workflow states from Linear");
            }
            Err(e) => {
                tracing::error!("Failed to sync workflow states: {e}");
            }
        }
    }

    async fn fetch_teams(&self) {
        match self.tracker.list_teams().await {
            Ok(teams) => {
                let count = teams.len();
                let _ = self.action_tx.send(Action::TeamsLoaded(teams));
                tracing::info!("Synced {count} teams from Linear");
            }
            Err(e) => {
                tracing::error!("Failed to sync teams: {e}");
            }
        }
    }

    async fn fetch_projects(&self) {
        match self.tracker.list_projects().await {
            Ok(projects) => {
                let count = projects.len();
                let _ = self.action_tx.send(Action::ProjectsLoaded(projects));
                tracing::info!("Synced {count} projects from Linear");
            }
            Err(e) => {
                tracing::error!("Failed to sync projects: {e}");
            }
        }
    }

    async fn fetch_labels(&self) {
        match self.tracker.list_labels().await {
            Ok(labels) => {
                let count = labels.len();
                let _ = self.action_tx.send(Action::LabelsLoaded(labels));
                tracing::info!("Synced {count} labels from Linear");
            }
            Err(e) => {
                tracing::error!("Failed to sync labels: {e}");
            }
        }
    }
}
