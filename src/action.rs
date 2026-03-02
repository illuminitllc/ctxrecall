use crate::tracker::types::{Issue, IssueStatus, IssueUpdate, Label, NewIssue, Team, Project};

#[derive(Debug, Clone, Default)]
pub struct DashboardStats {
    pub open_count: usize,
    pub closed_7d_count: usize,
    pub total_sessions: usize,
    pub active_session: Option<String>,
    pub last_session_issue: Option<String>,
    pub last_session_time: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Action {
    Tick,
    Render,
    Quit,
    Select,
    Back,
    OpenSearch,
    LaunchClaude(String),
    Refresh,
    OpenSettings,
    OpenCommandPalette,
    ShowHelp,
    HideHelp,

    // Data events from background tasks
    IssuesLoaded(Vec<Issue>),
    TeamsLoaded(Vec<Team>),
    ProjectsLoaded(Vec<Project>),
    WorkflowStatesLoaded(Vec<IssueStatus>),
    LabelsLoaded(Vec<Label>),
    StatusMessage(String),
    Error(String),

    // Detail view
    ShowIssueDetail(Issue),

    // Issue editing
    EditIssue(Issue),
    SaveIssueUpdate(String, IssueUpdate), // issue_id, update
    CreateIssue(NewIssue),
    IssueSaved(Issue),
    CycleStatus(String),
    ViewTranscripts(String),
    ViewDocuments(String),
    SearchQuery(String),
    SearchResults(Vec<crate::db::search_repo::SearchResult>),

    // Issue creation
    OpenNewIssue,

    // Filtering
    OpenTeamFilter,
    OpenProjectFilter,
    SetTeamFilter(Option<String>),                    // team name, None = clear
    SetProjectFilter(Option<String>, Option<String>), // (project, team), None = clear
}
