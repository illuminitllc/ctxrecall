use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crossterm::event::KeyModifiers;
use ratatui::layout::{Constraint, Layout};
use rusqlite::Connection;
use tokio::sync::mpsc;

use crate::action::{Action, DashboardStats};
use crate::db::config_repo::AccountRow;
use crate::tracker::types::{Label, Team, Project};
use crate::claude::context::{build_context_prompt, load_project_docs};
use crate::claude::session::ClaudeManager;
use crate::claude::transcript::TranscriptCaptureHandle;
use crate::components::Component;
use crate::components::account_picker::AccountPicker;
use crate::components::command_palette::CommandPalette;
use crate::components::dashboard::Dashboard;
use crate::components::document_viewer::DocumentViewer;
use crate::components::filter_picker::FilterPicker;
use crate::components::help_overlay::{HelpContext, HelpOverlay};
use crate::components::issue_create::IssueCreate;
use crate::components::issue_detail::IssueDetail;
use crate::components::issue_edit::IssueEdit;
use crate::components::issue_list::IssueList;
use crate::components::search::SearchOverlay;
use crate::components::settings::Settings;
use crate::components::status_bar::StatusBar;
use crate::components::transcript_viewer::TranscriptViewer;
use crate::config::hotkeys;
use crate::db;
use crate::event::{Event, EventHandler};
use crate::tmux::TmuxManager;
use crate::llm;
use crate::tracker::IssueTracker;
use crate::tracker::linear::LinearTracker;
use crate::tracker::sync::SyncManager;
use crate::tui::{self, Tui};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FocusPanel {
    IssueList,
    DetailPanel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BottomPanel {
    Dashboard,
    IssueDetail,
}

/// TUI pane size presets (percentage of tmux window width)
const PANE_SIZES: &[u16] = &[33, 50, 67];

pub struct App {
    running: bool,
    issue_list: IssueList,
    issue_detail: IssueDetail,
    issue_create: IssueCreate,
    issue_edit: IssueEdit,
    transcript_viewer: TranscriptViewer,
    document_viewer: DocumentViewer,
    search: SearchOverlay,
    command_palette: CommandPalette,
    settings: Settings,
    status_bar: StatusBar,
    dashboard: Dashboard,
    help_overlay: HelpOverlay,
    filter_picker: FilterPicker,
    account_picker: AccountPicker,
    focus: FocusPanel,
    bottom_panel: BottomPanel,
    action_rx: mpsc::UnboundedReceiver<Action>,
    action_tx: mpsc::UnboundedSender<Action>,
    db_conn: Connection,
    claude: Option<ClaudeManager>,
    tmux: Option<TmuxManager>,
    pane_size_index: usize,
    tracker: Option<Arc<dyn IssueTracker>>,
    sync_handle: Option<tokio::task::JoinHandle<()>>,
    data_dir: PathBuf,
    transcript_capture: Option<TranscriptCaptureHandle>,
    teams: Vec<Team>,
    projects: Vec<Project>,
    labels: Vec<Label>,
}

impl App {
    pub fn new(db_conn: Connection, data_dir: PathBuf) -> Self {
        let (action_tx, action_rx) = mpsc::unbounded_channel();

        let cached_issues = db::issue_repo::load_cached_issues(&db_conn).unwrap_or_default();
        let cached_states = db::issue_repo::load_workflow_states(&db_conn).unwrap_or_default();
        let cached_teams = db::issue_repo::load_cached_teams(&db_conn).unwrap_or_default();
        let cached_projects = db::issue_repo::load_cached_projects(&db_conn).unwrap_or_default();
        let cached_labels = db::issue_repo::load_cached_labels(&db_conn).unwrap_or_default();
        let mut issue_list = IssueList::new();
        if !cached_issues.is_empty() {
            issue_list.update(&Action::IssuesLoaded(cached_issues));
        }
        if !cached_states.is_empty() {
            issue_list.set_workflow_states(cached_states);
        }

        let (claude, tmux) = if TmuxManager::is_inside_tmux() {
            match TmuxManager::new() {
                Ok(tmux) => {
                    // Claude manager is created without a pane — the pane
                    // is spawned on-demand when the user first launches a session.
                    let claude = if !tmux.claude_pane.is_empty() {
                        Some(ClaudeManager::new(tmux.claude_pane.clone()))
                    } else {
                        None
                    };
                    (claude, Some(tmux))
                }
                Err(e) => {
                    tracing::warn!("Could not init tmux manager: {e}");
                    (None, None)
                }
            }
        } else {
            (None, None)
        };

        let transcript_dir = data_dir.join("transcripts");

        // Start LLM summarizer if configured
        if let Some(provider) = llm::create_provider(&db_conn) {
            tracing::info!("LLM provider configured: {}", provider.name());
            let summarizer = llm::summarizer::Summarizer::new(
                provider,
                transcript_dir.clone(),
                action_tx.clone(),
            );
            summarizer.start(Duration::from_secs(120));
        }

        Self {
            running: true,
            issue_list,
            issue_detail: IssueDetail::new(),
            issue_create: IssueCreate::new(),
            issue_edit: IssueEdit::new(),
            transcript_viewer: TranscriptViewer::new(transcript_dir),
            document_viewer: DocumentViewer::new(),
            search: SearchOverlay::new(),
            command_palette: CommandPalette::new(),
            settings: Settings::new(),
            status_bar: StatusBar::new(),
            dashboard: Dashboard::new(),
            help_overlay: HelpOverlay::new(),
            filter_picker: FilterPicker::new(),
            account_picker: AccountPicker::new(),
            focus: FocusPanel::IssueList,
            bottom_panel: BottomPanel::Dashboard,
            action_rx,
            action_tx,
            db_conn,
            claude,
            tmux,
            pane_size_index: 1, // Start at 50%
            tracker: None,
            sync_handle: None,
            data_dir,
            transcript_capture: None,
            teams: cached_teams,
            projects: cached_projects,
            labels: cached_labels,
        }
    }

    pub fn start_sync(&mut self, api_key: String) {
        if let Some(handle) = self.sync_handle.take() {
            handle.abort();
        }
        let tracker: Arc<dyn IssueTracker> = Arc::new(LinearTracker::new(api_key));
        self.tracker = Some(tracker.clone());
        let sync = SyncManager::new(tracker, self.action_tx.clone());
        self.sync_handle = Some(sync.start_background_sync(Duration::from_secs(120)));
    }

    pub async fn run(&mut self) -> color_eyre::Result<()> {
        let mut terminal = tui::init()?;
        let mut events = EventHandler::new(Duration::from_millis(250));

        self.render(&mut terminal)?;

        while self.running {
            while let Ok(action) = self.action_rx.try_recv() {
                self.handle_action(action);
            }

            if let Some(event) = events.next().await {
                let action = match event {
                    Event::Key(key) => self.dispatch_key(key),
                    Event::Tick => None,
                    Event::Resize(_, _) => None,
                };

                if let Some(action) = action {
                    self.handle_action(action);
                }

                self.render(&mut terminal)?;
            }
        }

        // Stop transcript capture
        if let Some(capture) = self.transcript_capture.take() {
            capture.stop();
        }

        // Clean shutdown: exit any active Claude session
        if let Some(claude) = &mut self.claude {
            if let Some(sid) = claude.active_claude_session_id() {
                tracing::info!("Shutting down — ending Claude session {sid}");
            }
            if let Err(e) = claude.exit_current_session(&self.db_conn) {
                tracing::warn!("Failed to cleanly exit Claude session: {e}");
            }
        }

        tui::restore()?;
        Ok(())
    }

    fn dispatch_key(&mut self, key: crossterm::event::KeyEvent) -> Option<Action> {
        // Overlay priority chain — highest priority first
        if self.help_overlay.is_visible() {
            return self.help_overlay.handle_key_event(key);
        }
        if self.command_palette.is_visible() {
            return self.command_palette.handle_key_event(key);
        }
        if self.account_picker.is_visible() {
            return self.account_picker.handle_key_event(key);
        }
        if self.settings.is_visible() {
            return self.settings.handle_key_event(key);
        }
        if self.search.is_visible() {
            return self.search.handle_key_event(key);
        }
        if self.filter_picker.is_visible() {
            return self.filter_picker.handle_key_event(key);
        }
        if self.document_viewer.is_visible() {
            return self.document_viewer.handle_key_event(key);
        }
        if self.transcript_viewer.is_visible() {
            return self.transcript_viewer.handle_key_event(key);
        }
        if self.issue_create.is_visible() {
            return self.issue_create.handle_key_event(key);
        }
        if self.issue_edit.is_visible() {
            return self.issue_edit.handle_key_event(key);
        }

        // No overlay active — route to focused panel
        self.handle_focused_key(key)
    }

    fn handle_focused_key(&mut self, key: crossterm::event::KeyEvent) -> Option<Action> {
        // Global Ctrl combos
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return match key.code {
                crossterm::event::KeyCode::Char('s') => Some(Action::OpenSettings),
                crossterm::event::KeyCode::Char('p') => Some(Action::OpenCommandPalette),
                crossterm::event::KeyCode::Char('r') => {
                    self.cycle_pane_size();
                    None
                }
                _ => None,
            };
        }

        // Tab switches focus between panels when detail is showing
        if key.code == crossterm::event::KeyCode::Tab
            && self.bottom_panel == BottomPanel::IssueDetail
        {
            self.focus = match self.focus {
                FocusPanel::IssueList => FocusPanel::DetailPanel,
                FocusPanel::DetailPanel => FocusPanel::IssueList,
            };
            return None;
        }

        // 'h' for help
        if key.code == crossterm::event::KeyCode::Char('h') {
            return Some(Action::ShowHelp);
        }

        // 'n' for new issue
        if key.code == crossterm::event::KeyCode::Char('n') {
            return Some(Action::OpenNewIssue);
        }

        // 'a' for quick account switch
        if key.code == crossterm::event::KeyCode::Char('a') {
            let accounts = db::config_repo::list_accounts(&self.db_conn, &["linear"]).unwrap_or_default();
            if accounts.len() > 1 {
                let picker_accounts: Vec<crate::components::account_picker::Account> = accounts
                    .into_iter()
                    .map(|a| crate::components::account_picker::Account {
                        id: a.id,
                        name: a.name,
                        provider: a.provider,
                        is_active: a.is_active,
                    })
                    .collect();
                self.account_picker.show(picker_accounts);
            } else {
                let _ = self.action_tx.send(Action::StatusMessage("Only one account configured".into()));
            }
            return None;
        }

        // Global filter keys
        if key.code == crossterm::event::KeyCode::Char('t') {
            return Some(Action::OpenTeamFilter);
        }
        if key.code == crossterm::event::KeyCode::Char('p') {
            return Some(Action::OpenProjectFilter);
        }

        // Delegate to focused component
        match self.focus {
            FocusPanel::IssueList => self.issue_list.handle_key_event(key),
            FocusPanel::DetailPanel => self.issue_detail.handle_key_event(key),
        }
    }

    fn handle_action(&mut self, action: Action) {
        if let Action::WorkflowStatesLoaded(ref states) = action {
            if let Err(e) = db::issue_repo::upsert_workflow_states(&self.db_conn, states) {
                tracing::error!("Failed to cache workflow states: {e}");
            }
        }

        if let Action::TeamsLoaded(ref teams) = action {
            self.teams = teams.clone();
        }

        if let Action::ProjectsLoaded(ref projects) = action {
            self.projects = projects.clone();
        }

        if let Action::LabelsLoaded(ref labels) = action {
            self.labels = labels.clone();
            if let Err(e) = db::issue_repo::upsert_labels(&self.db_conn, labels) {
                tracing::error!("Failed to cache labels: {e}");
            }
        }

        if let Action::IssuesLoaded(ref issues) = action {
            if let Err(e) = db::issue_repo::upsert_issues(&self.db_conn, issues) {
                tracing::error!("Failed to cache issues: {e}");
            }
            // Index issues for search
            for issue in issues {
                let content = format!(
                    "{} {} {}",
                    issue.title,
                    issue.description.as_deref().unwrap_or(""),
                    issue.labels.join(" ")
                );
                let _ = db::search_repo::index_content(
                    &self.db_conn,
                    "issue",
                    &issue.id,
                    &issue.id,
                    &issue.identifier,
                    &content,
                );
            }

            // Compute dashboard stats
            self.update_dashboard_stats(issues);
        }

        // Dispatch to components
        if let Some(f) = self.issue_list.update(&action) {
            self.handle_action(f);
            return;
        }
        if let Some(f) = self.issue_detail.update(&action) {
            self.handle_action(f);
            return;
        }
        if let Some(f) = self.issue_create.update(&action) {
            self.handle_action(f);
            return;
        }
        if let Some(f) = self.issue_edit.update(&action) {
            self.handle_action(f);
            return;
        }
        if let Some(f) = self.transcript_viewer.update(&action) {
            self.handle_action(f);
            return;
        }
        if let Some(f) = self.document_viewer.update(&action) {
            self.handle_action(f);
            return;
        }
        if let Some(f) = self.filter_picker.update(&action) {
            self.handle_action(f);
            return;
        }
        if let Some(f) = self.search.update(&action) {
            self.handle_action(f);
            return;
        }
        if let Some(f) = self.command_palette.update(&action) {
            self.handle_action(f);
            return;
        }
        if let Some(f) = self.settings.update(&action) {
            self.handle_action(f);
            return;
        }
        self.status_bar.update(&action);

        match action {
            Action::Quit => self.running = false,
            Action::ShowIssueDetail(issue) => {
                self.issue_detail.set_issue(issue);
                self.bottom_panel = BottomPanel::IssueDetail;
                self.focus = FocusPanel::DetailPanel;
            }
            Action::Back => {
                if self.bottom_panel == BottomPanel::IssueDetail {
                    self.issue_detail.clear();
                    self.bottom_panel = BottomPanel::Dashboard;
                    self.focus = FocusPanel::IssueList;
                }
            }
            Action::OpenNewIssue => {
                let workflow_states =
                    db::issue_repo::load_workflow_states(&self.db_conn).unwrap_or_default();
                self.issue_create.show(
                    &self.teams,
                    &self.projects,
                    self.issue_list.all_issues(),
                    &workflow_states,
                    &self.labels,
                );
            }
            Action::EditIssue(issue) => self.issue_edit.show(issue),
            Action::LaunchClaude(issue_id) => self.launch_claude(&issue_id),
            Action::CycleStatus(issue_id) => self.cycle_issue_status(&issue_id),
            Action::SaveIssueUpdate(id, update) => self.save_issue_update(id, update),
            Action::CreateIssue(new_issue) => self.create_issue(new_issue),
            Action::IssueSaved(ref issue) => {
                if let Err(e) = db::issue_repo::upsert_issues(&self.db_conn, &[issue.clone()]) {
                    tracing::error!("Failed to cache updated issue: {e}");
                }
            }
            Action::ViewTranscripts(issue_id) => self.transcript_viewer.show(&issue_id),
            Action::ViewDocuments(issue_id) => {
                let docs =
                    db::document_repo::list_documents_for_issue(&self.db_conn, &issue_id)
                        .unwrap_or_default();
                self.document_viewer.show(&issue_id, docs);
            }
            Action::CreateDocument { issue_id, doc_type, title } => {
                let issue = self.issue_list.find_issue(&issue_id).cloned();
                let identifier = issue
                    .as_ref()
                    .map(|i| i.identifier.clone())
                    .unwrap_or_else(|| issue_id.clone());
                let working_dir = issue.as_ref().and_then(|i| self.resolve_working_dir(i));

                let filename = db::document_repo::doc_filename(&identifier, &doc_type, &title);
                let docs_dir = db::document_repo::resolve_docs_dir(
                    working_dir.as_deref(),
                    &self.data_dir,
                    &issue_id,
                );

                // Write file to disk
                let file_path = match db::document_repo::write_doc_file(
                    &docs_dir,
                    &filename,
                    &title,
                    &doc_type,
                    "",
                ) {
                    Ok(p) => Some(p.to_string_lossy().to_string()),
                    Err(e) => {
                        tracing::error!("Failed to write doc file: {e}");
                        None
                    }
                };

                // Create in DB
                match db::document_repo::create_document(
                    &self.db_conn,
                    &issue_id,
                    &doc_type,
                    &title,
                    "",
                    file_path.as_deref(),
                ) {
                    Ok(doc) => {
                        // Index for FTS
                        let _ = db::search_repo::index_content(
                            &self.db_conn,
                            "document",
                            &doc.id,
                            &issue_id,
                            &title,
                            &format!("{} {}", title, doc_type),
                        );
                        let _ = self.action_tx.send(Action::DocumentCreated(doc));
                        let _ = self.action_tx.send(Action::StatusMessage(
                            format!("Document created: {filename}")
                        ));
                    }
                    Err(e) => {
                        let _ = self.action_tx.send(Action::Error(
                            format!("Failed to create document: {e}")
                        ));
                    }
                }
            }
            Action::SaveDocumentContent { doc_id, content } => {
                match db::document_repo::get_document(&self.db_conn, &doc_id) {
                    Ok(doc) => {
                        if let Err(e) = db::document_repo::update_document(
                            &self.db_conn,
                            &doc_id,
                            &doc.title,
                            &content,
                        ) {
                            let _ = self.action_tx.send(Action::Error(
                                format!("Failed to save document: {e}")
                            ));
                            return;
                        }
                        // Update file on disk if file_path is set
                        if let Some(ref fp) = doc.file_path {
                            if let Err(e) = std::fs::write(fp, &content) {
                                tracing::warn!("Failed to update doc file {fp}: {e}");
                            }
                        }
                        // Update FTS index
                        let _ = db::search_repo::index_content(
                            &self.db_conn,
                            "document",
                            &doc_id,
                            &doc.issue_id,
                            &doc.title,
                            &content,
                        );
                        let _ = self.action_tx.send(Action::StatusMessage(
                            "Document saved".into()
                        ));
                    }
                    Err(e) => {
                        let _ = self.action_tx.send(Action::Error(
                            format!("Failed to load document: {e}")
                        ));
                    }
                }
            }
            Action::DocumentCreated(_) => {} // handled in document_viewer.update()
            Action::OpenSearch => self.search.show(None),
            Action::OpenCommandPalette => self.command_palette.show(),
            Action::OpenSettings => {
                let hk = hotkeys::load_hotkeys(&self.db_conn)
                    .unwrap_or_default()
                    .into_values()
                    .collect();
                let accounts = db::config_repo::list_accounts(
                    &self.db_conn,
                    &["linear", "claude", "openai", "ollama"],
                )
                .unwrap_or_default();
                self.settings.set_accounts(accounts);
                self.settings.show(hk);
            }
            Action::ShowHelp => {
                let ctx = match self.focus {
                    FocusPanel::IssueList => HelpContext::IssueList,
                    FocusPanel::DetailPanel => HelpContext::DetailPanel,
                };
                self.help_overlay.show(ctx);
            }
            Action::HideHelp => {}
            Action::SearchQuery(query) => {
                if query.len() >= 2 {
                    let scope = self.search.scope_issue_id().map(|s| s.to_string());
                    match db::search_repo::search(
                        &self.db_conn,
                        &query,
                        scope.as_deref(),
                        20,
                    ) {
                        Ok(results) => self.search.set_results(results),
                        Err(e) => tracing::error!("Search failed: {e}"),
                    }
                }
            }
            Action::OpenTeamFilter => {
                let mut teams: Vec<String> = self.teams.iter().map(|t| t.name.clone()).collect();
                if teams.is_empty() {
                    // Fallback: derive from loaded issues
                    teams = self.issue_list.unique_teams();
                }
                teams.sort();
                teams.dedup();
                let current = self.issue_list.team_filter().map(|s| s.as_str());
                self.filter_picker.show_teams(teams, current);
            }
            Action::OpenProjectFilter => {
                let current = self.issue_list.project_filter().map(|s| s.as_str());
                self.filter_picker.show_projects(
                    self.issue_list.all_issues(),
                    &self.teams,
                    &self.projects,
                    current,
                );
            }
            Action::SetTeamFilter(team) => {
                self.issue_list.set_team_filter(team);
            }
            Action::SetProjectFilter(project, team) => {
                self.issue_list.set_project_filter(project, team);
            }
            Action::SaveAccount { id, name, provider, api_key, model, ollama_url } => {
                let result = if let Some(existing_id) = &id {
                    db::config_repo::update_account(&self.db_conn, existing_id, &name, &api_key)
                        .map(|_| existing_id.clone())
                } else {
                    db::config_repo::insert_account(&self.db_conn, &name, &provider, &api_key)
                };
                match result {
                    Ok(account_id) => {
                        // Save LLM extras if applicable
                        if provider != "linear" {
                            let _ = db::config_repo::set_account_llm_config(
                                &self.db_conn,
                                &account_id,
                                model.as_deref(),
                                ollama_url.as_deref(),
                            );
                        }
                        // If this is the first account of its type, auto-activate it
                        let group: &[&str] = if provider == "linear" {
                            &["linear"]
                        } else {
                            &["claude", "openai", "ollama"]
                        };
                        let accounts = db::config_repo::list_accounts(&self.db_conn, group).unwrap_or_default();
                        if accounts.len() == 1 || !accounts.iter().any(|a| a.is_active) {
                            let _ = db::config_repo::set_active_account(&self.db_conn, &account_id, group);
                        }
                        let _ = self.action_tx.send(Action::StatusMessage(
                            if id.is_some() { "Account updated".into() } else { "Account created".into() }
                        ));
                        let _ = self.action_tx.send(Action::LoadAccounts);
                    }
                    Err(e) => {
                        let _ = self.action_tx.send(Action::Error(format!("Failed to save account: {e}")));
                    }
                }
            }
            Action::DeleteAccount(id) => {
                if let Err(e) = db::config_repo::delete_account(&self.db_conn, &id) {
                    let _ = self.action_tx.send(Action::Error(format!("Failed to delete account: {e}")));
                } else {
                    let _ = self.action_tx.send(Action::StatusMessage("Account deleted".into()));
                    let _ = self.action_tx.send(Action::LoadAccounts);
                }
            }
            Action::SwitchAccount(id) => {
                match db::config_repo::get_account(&self.db_conn, &id) {
                    Ok(Some(account)) => {
                        let group: &[&str] = if account.provider == "linear" {
                            &["linear"]
                        } else {
                            &["claude", "openai", "ollama"]
                        };
                        if let Err(e) = db::config_repo::set_active_account(&self.db_conn, &id, group) {
                            let _ = self.action_tx.send(Action::Error(format!("Failed to switch account: {e}")));
                        } else if account.provider == "linear" {
                            // Restart sync with new key
                            self.start_sync(account.api_key.clone());
                            let _ = self.action_tx.send(Action::StatusMessage(
                                format!("Switched to: {}", account.name)
                            ));
                        } else {
                            // LLM switch: update global config keys
                            let _ = db::config_repo::set_config(&self.db_conn, "llm_provider", &account.provider);
                            let _ = db::config_repo::set_config(&self.db_conn, "llm_api_key", &account.api_key);
                            let (model, ollama_url) = db::config_repo::get_account_llm_config(&self.db_conn, &id);
                            if let Some(m) = &model {
                                let _ = db::config_repo::set_config(&self.db_conn, "llm_model", m);
                            }
                            if let Some(u) = &ollama_url {
                                let _ = db::config_repo::set_config(&self.db_conn, "llm_ollama_url", u);
                            }
                            let _ = self.action_tx.send(Action::StatusMessage(
                                format!("LLM switched to: {} (restart to take effect)", account.name)
                            ));
                        }
                        let _ = self.action_tx.send(Action::LoadAccounts);
                    }
                    Ok(None) => {
                        let _ = self.action_tx.send(Action::Error("Account not found".into()));
                    }
                    Err(e) => {
                        let _ = self.action_tx.send(Action::Error(format!("Failed to load account: {e}")));
                    }
                }
            }
            Action::LoadAccounts => {
                let accounts = db::config_repo::list_accounts(
                    &self.db_conn,
                    &["linear", "claude", "openai", "ollama"],
                )
                .unwrap_or_default();
                self.settings.set_accounts(accounts);
            }
            Action::AccountsLoaded(_) => {} // handled in settings.update()
            Action::Refresh => {
                let _ = self
                    .action_tx
                    .send(Action::StatusMessage("Refreshing...".into()));
            }
            _ => {}
        }
    }

    fn update_dashboard_stats(&mut self, issues: &[crate::tracker::types::Issue]) {
        let open_count = issues
            .iter()
            .filter(|i| {
                !matches!(
                    i.status.as_str(),
                    "Done" | "Completed" | "Canceled" | "Cancelled"
                )
            })
            .count();

        let now = chrono::Utc::now();
        let seven_days_ago = now - chrono::Duration::days(7);
        let closed_7d_count = issues
            .iter()
            .filter(|i| {
                matches!(i.status.as_str(), "Done" | "Completed")
                    && i.updated_at > seven_days_ago
            })
            .count();

        let session_stats =
            db::session_repo::get_dashboard_session_stats(&self.db_conn).unwrap_or_default();

        self.dashboard.set_stats(DashboardStats {
            open_count,
            closed_7d_count,
            total_sessions: session_stats.total_sessions,
            active_session: session_stats.active_session_issue,
            last_session_issue: session_stats.last_session_issue,
            last_session_time: session_stats.last_session_time,
        });
    }

    fn cycle_issue_status(&self, issue_id: &str) {
        let Some(issue) = self.issue_list.find_issue(issue_id) else {
            return;
        };
        let team_id = issue.team_id.as_deref().unwrap_or("");
        let cycle = self.issue_list.status_cycle_for_team(team_id);
        if cycle.is_empty() {
            let _ = self
                .action_tx
                .send(Action::Error("No status IDs available — refresh first".into()));
            return;
        }
        // Find current position in cycle, advance to next
        let current_pos = cycle.iter().position(|(name, _)| *name == issue.status);
        let next = match current_pos {
            Some(pos) => cycle[(pos + 1) % cycle.len()],
            None => cycle[0],
        };
        let (next_name, next_id) = next;
        let update = crate::tracker::types::IssueUpdate {
            status_id: Some(next_id.to_string()),
            ..Default::default()
        };
        let _ = self.action_tx.send(Action::StatusMessage(format!(
            "Changing status to {next_name}..."
        )));
        self.save_issue_update(issue_id.to_string(), update);
    }

    fn save_issue_update(&self, issue_id: String, update: crate::tracker::types::IssueUpdate) {
        let Some(tracker) = self.tracker.clone() else {
            let _ = self.action_tx.send(Action::Error("No tracker configured".into()));
            return;
        };
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            let _ = tx.send(Action::StatusMessage("Saving...".into()));
            match tracker.update_issue(&issue_id, &update).await {
                Ok(issue) => {
                    let _ = tx.send(Action::IssueSaved(issue));
                    let _ = tx.send(Action::StatusMessage("Issue saved".into()));
                }
                Err(e) => {
                    tracing::error!("Failed to update issue: {e}");
                    let _ = tx.send(Action::Error(format!("Save failed: {e}")));
                }
            }
        });
    }

    fn create_issue(&self, new_issue: crate::tracker::types::NewIssue) {
        let Some(tracker) = self.tracker.clone() else {
            let _ = self.action_tx.send(Action::Error("No tracker configured".into()));
            return;
        };
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            let _ = tx.send(Action::StatusMessage("Creating issue...".into()));
            match tracker.create_issue(&new_issue).await {
                Ok(issue) => {
                    let _ = tx.send(Action::IssueSaved(issue));
                    let _ = tx.send(Action::StatusMessage("Issue created".into()));
                }
                Err(e) => {
                    tracing::error!("Failed to create issue: {e}");
                    let _ = tx.send(Action::Error(format!("Create failed: {e}")));
                }
            }
        });
    }

    /// Resolve working directory for an issue: project first, then team as fallback.
    /// Each level checks by ID first (stable), then by name (ergonomic).
    fn resolve_working_dir(&self, issue: &crate::tracker::types::Issue) -> Option<String> {
        // Try project by ID first
        if let Some(project_id) = &issue.project_id {
            if let Ok(Some(dir)) =
                db::config_repo::get_config(&self.db_conn, &format!("project_dir:{project_id}"))
            {
                return Some(dir);
            }
        }
        // Then project by name
        if let Some(project) = &issue.project {
            if let Ok(Some(dir)) =
                db::config_repo::get_config(&self.db_conn, &format!("project_dir:{project}"))
            {
                return Some(dir);
            }
        }
        // Try team by ID first
        if let Some(team_id) = &issue.team_id {
            if let Ok(Some(dir)) =
                db::config_repo::get_config(&self.db_conn, &format!("team_dir:{team_id}"))
            {
                return Some(dir);
            }
        }
        // Then team by name
        if let Some(team) = &issue.team {
            if let Ok(Some(dir)) =
                db::config_repo::get_config(&self.db_conn, &format!("team_dir:{team}"))
            {
                return Some(dir);
            }
        }
        None
    }

    fn launch_claude(&mut self, issue_id: &str) {
        // Create Claude pane on demand if we're in tmux but haven't split yet
        if self.claude.is_none() {
            if let Some(tmux) = &mut self.tmux {
                match tmux.create_claude_pane(33) {
                    Ok(()) => {
                        let pane_id = tmux.claude_pane.clone();
                        self.claude = Some(ClaudeManager::new(pane_id.clone()));
                        self.pane_size_index = 0; // 33%

                        // Start transcript capture now that the pane exists
                        let transcript_dir = self.data_dir.join("transcripts");
                        self.transcript_capture = Some(TranscriptCaptureHandle::new(
                            pane_id,
                            transcript_dir,
                            self.action_tx.clone(),
                            5,
                        ));
                    }
                    Err(e) => {
                        let _ = self
                            .action_tx
                            .send(Action::Error(format!("Failed to create Claude pane: {e}")));
                        return;
                    }
                }
            } else {
                let _ = self.action_tx.send(Action::Error(
                    "Claude unavailable: not running in tmux".into(),
                ));
                return;
            }
        }

        let issue = self
            .issue_list
            .selected_issue()
            .filter(|i| i.id == issue_id)
            .cloned();

        let identifier = issue
            .as_ref()
            .map(|i| i.identifier.clone())
            .unwrap_or_else(|| issue_id.to_string());

        let working_dir = issue.as_ref().and_then(|i| self.resolve_working_dir(i));

        let claude = self.claude.as_mut().unwrap();

        // Check if this is a new session (not already active for same issue)
        let is_new_session = claude.active_issue_id() != Some(issue_id);

        match claude.launch_for_issue(
            &self.db_conn,
            issue_id,
            &identifier,
            working_dir.as_deref(),
        ) {
            Ok(()) => {
                self.issue_list
                    .set_active_claude_issue(Some(issue_id.to_string()));
                // Start capturing transcript for this issue
                if let Some(capture) = &self.transcript_capture {
                    capture.set_issue(issue_id);
                }
                // Shrink TUI pane to 33% to give Claude more room, then focus Claude pane
                if let Some(tmux) = &self.tmux {
                    if let Err(e) = tmux.resize_tui_pane(33) {
                        tracing::warn!("Failed to resize pane: {e}");
                    } else {
                        self.pane_size_index = 0; // 33%
                    }
                    if let Err(e) = tmux.focus_claude_pane() {
                        tracing::warn!("Failed to focus Claude pane: {e}");
                    }
                }

                // Inject context for new sessions
                if is_new_session {
                    self.schedule_context_injection(issue.as_ref(), issue_id);
                }

                let _ = self.action_tx.send(Action::StatusMessage(format!(
                    "Claude active for {identifier}"
                )));
            }
            Err(e) => {
                tracing::error!("Failed to launch Claude: {e}");
                let _ = self
                    .action_tx
                    .send(Action::Error(format!("Claude launch failed: {e}")));
            }
        }
    }

    fn schedule_context_injection(
        &self,
        issue: Option<&crate::tracker::types::Issue>,
        issue_id: &str,
    ) {
        let Some(issue) = issue.cloned() else {
            return;
        };
        let Some(claude) = &self.claude else {
            return;
        };

        let summaries = self.load_recent_summaries(issue_id);
        let documents =
            db::document_repo::list_documents_for_issue(&self.db_conn, issue_id).unwrap_or_default();

        // Load project-level docs from the working directory
        let working_dir = self.resolve_working_dir(&issue);
        let project_docs = working_dir
            .as_ref()
            .map(|dir| load_project_docs(std::path::Path::new(dir)))
            .unwrap_or_default();

        let Some(prompt) = build_context_prompt(&issue, &summaries, &documents, &project_docs) else {
            tracing::debug!("No context to inject for {}", issue.identifier);
            return;
        };

        let pane = claude.pane_id().to_string();
        let identifier = issue.identifier.clone();

        tokio::spawn(async move {
            // Wait for Claude CLI to initialize
            tokio::time::sleep(Duration::from_secs(2)).await;

            match ClaudeManager::inject_context(&pane, &prompt) {
                Ok(()) => tracing::info!("Context injected for {identifier}"),
                Err(e) => tracing::warn!("Context injection failed for {identifier}: {e}"),
            }
        });
    }

    /// Load recent `.summary.md` files from the transcript directory for an issue,
    /// sorted newest-first.
    fn load_recent_summaries(&self, issue_id: &str) -> Vec<String> {
        let dir = self.data_dir.join("transcripts").join(issue_id);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return Vec::new();
        };

        let mut files: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map_or(false, |n| n.ends_with(".summary.md"))
            })
            .collect();

        // Sort by filename descending (filenames are timestamped: YYYYMMDD_HHMMSS.summary.md)
        files.sort_by(|a, b| b.file_name().cmp(&a.file_name()));

        files
            .iter()
            .take(3)
            .filter_map(|e| std::fs::read_to_string(e.path()).ok())
            .filter(|s| !s.trim().is_empty())
            .collect()
    }

    fn cycle_pane_size(&mut self) {
        if let Some(tmux) = &self.tmux {
            self.pane_size_index = (self.pane_size_index + 1) % PANE_SIZES.len();
            let pct = PANE_SIZES[self.pane_size_index];
            if let Err(e) = tmux.resize_tui_pane(pct) {
                tracing::warn!("Failed to resize pane: {e}");
            } else {
                let _ = self
                    .action_tx
                    .send(Action::StatusMessage(format!("Pane size: {pct}%")));
            }
        }
    }

    fn render(&mut self, terminal: &mut Tui) -> color_eyre::Result<()> {
        terminal.draw(|frame| {
            let chunks = Layout::vertical([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
                Constraint::Length(1),
            ])
            .split(frame.area());

            // Top: issue list (highlight border if focused)
            self.issue_list.render(frame, chunks[0]);

            // Bottom: dashboard or issue detail
            match self.bottom_panel {
                BottomPanel::Dashboard => self.dashboard.render(frame, chunks[1]),
                BottomPanel::IssueDetail => self.issue_detail.render(frame, chunks[1]),
            }

            // Status bar
            self.status_bar.render(frame, chunks[2]);

            // Overlays (order matters — last rendered is on top)
            self.issue_create.render(frame, frame.area());
            self.issue_edit.render(frame, frame.area());
            self.transcript_viewer.render(frame, frame.area());
            self.document_viewer.render(frame, frame.area());
            self.filter_picker.render(frame, frame.area());
            self.search.render(frame, frame.area());
            self.settings.render(frame, frame.area());
            self.account_picker.render(frame, frame.area());
            self.command_palette.render(frame, frame.area());
            self.help_overlay.render(frame, frame.area());
        })?;
        Ok(())
    }
}
