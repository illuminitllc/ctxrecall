use crate::db::config_repo::AccountRow;
use crate::db::document_repo::Document;
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
    SetStatus(String, String), // (issue_id, status_name)
    OpenExternalEditor { field_id: String, current_value: String },
    ExternalEditorResult { field_id: String, new_value: String },
    ViewTranscripts(String),
    ViewDocuments(String),
    CreateDocument { issue_id: String, doc_type: String, title: String },
    SaveDocumentContent { doc_id: String, content: String },
    DocumentCreated(Document),

    SearchQuery(String),
    SearchResults(Vec<crate::db::search_repo::SearchResult>),
    SearchSelect { source_type: String, source_id: String, issue_id: String },

    // Issue creation
    OpenNewIssue,

    // Filtering
    OpenTeamFilter,
    OpenProjectFilter,
    SetTeamFilter(Option<String>),                    // team name, None = clear
    SetProjectFilter(Option<String>, Option<String>), // (project, team), None = clear

    // Account management
    SaveAccount {
        id: Option<String>,        // None = create, Some = update
        name: String,
        provider: String,
        api_key: String,
        model: Option<String>,     // LLM only
        ollama_url: Option<String>, // LLM only
    },
    DeleteAccount(String),          // account id
    SwitchAccount(String),          // account id
    LoadAccounts,
    AccountsLoaded(Vec<AccountRow>),

    // Directory mappings
    SaveDirectoryMapping { mapping_type: String, name: String, path: String },
    DeleteDirectoryMapping { key: String },
    LoadDirectoryMappings,
    DirectoryMappingsLoaded(Vec<(String, String, String)>), // (type, name, path)

    // Theme
    SetTheme(String), // theme name

    // Branch tracking
    OpenBranchPicker(String),              // issue_id
    SetBranch(String, String),             // (issue_id, branch_name)
    ClearBranch(String),                   // issue_id
    CreateAndSetBranch(String, String),    // (issue_id, branch_name)
}
