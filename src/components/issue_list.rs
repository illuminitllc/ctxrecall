use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};

use crate::action::Action;
use crate::tracker::types::{Issue, IssueStatus};

use super::Component;

pub struct IssueList {
    all_issues: Vec<Issue>,
    filtered: Vec<Issue>,
    state: ListState,
    active_claude_issue_id: Option<String>,
    status_filter: Option<String>,
    available_statuses: Vec<String>,
    workflow_states: Vec<IssueStatus>,
    team_filter: Option<String>,
    project_filter: Option<String>,
}

impl IssueList {
    pub fn new() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self {
            all_issues: Vec::new(),
            filtered: Vec::new(),
            state,
            active_claude_issue_id: None,
            status_filter: None,
            available_statuses: Vec::new(),
            workflow_states: Vec::new(),
            team_filter: None,
            project_filter: None,
        }
    }

    fn selected_index(&self) -> usize {
        self.state.selected().unwrap_or(0)
    }

    fn move_up(&mut self) {
        let i = self.selected_index();
        if i > 0 {
            self.state.select(Some(i - 1));
        }
    }

    fn move_down(&mut self) {
        let i = self.selected_index();
        let max = if self.filtered.is_empty() {
            0
        } else {
            self.filtered.len() - 1
        };
        if i < max {
            self.state.select(Some(i + 1));
        }
    }

    pub fn set_active_claude_issue(&mut self, issue_id: Option<String>) {
        self.active_claude_issue_id = issue_id;
    }

    pub fn selected_issue(&self) -> Option<&Issue> {
        self.filtered.get(self.selected_index())
    }

    pub fn find_issue(&self, id: &str) -> Option<&Issue> {
        self.all_issues.iter().find(|i| i.id == id)
    }

    pub fn set_workflow_states(&mut self, states: Vec<IssueStatus>) {
        self.workflow_states = states;
    }

    pub fn unique_teams(&self) -> Vec<String> {
        let mut teams: Vec<String> = Vec::new();
        for issue in &self.all_issues {
            if let Some(ref t) = issue.team {
                if !teams.contains(t) {
                    teams.push(t.clone());
                }
            }
        }
        teams.sort();
        teams
    }

    pub fn all_issues(&self) -> &[Issue] {
        &self.all_issues
    }

    pub fn team_filter(&self) -> Option<&String> {
        self.team_filter.as_ref()
    }

    pub fn project_filter(&self) -> Option<&String> {
        self.project_filter.as_ref()
    }

    pub fn set_team_filter(&mut self, team: Option<String>) {
        self.team_filter = team;
        self.project_filter = None; // clear project when team changes
        self.apply_filter();
    }

    pub fn set_project_filter(&mut self, project: Option<String>, team: Option<String>) {
        self.project_filter = project;
        if team.is_some() {
            self.team_filter = team;
        }
        self.apply_filter();
    }

    /// Returns workflow states for a given team, sorted by position.
    /// Falls back to deriving from loaded issues if no workflow states cached.
    pub fn status_cycle_for_team(&self, team_id: &str) -> Vec<(&str, &str)> {
        let team_states: Vec<_> = self
            .workflow_states
            .iter()
            .filter(|s| s.team_id == team_id)
            .collect();

        if !team_states.is_empty() {
            // Already sorted by position from DB query
            return team_states
                .iter()
                .map(|s| (s.name.as_str(), s.id.as_str()))
                .collect();
        }

        // Fallback: derive from loaded issues (old behavior)
        self.status_cycle_from_issues()
    }

    fn status_cycle_from_issues(&self) -> Vec<(&str, &str)> {
        let order = |s: &str| match s {
            "Backlog" => 0,
            "Todo" => 1,
            "In Progress" => 2,
            "Done" | "Completed" => 3,
            "Canceled" | "Cancelled" => 4,
            _ => 2,
        };
        let mut seen: Vec<(&str, &str)> = Vec::new();
        for issue in &self.all_issues {
            if let Some(ref sid) = issue.status_id {
                if !seen.iter().any(|(name, _)| *name == issue.status.as_str()) {
                    seen.push((issue.status.as_str(), sid.as_str()));
                }
            }
        }
        seen.sort_by_key(|(name, _)| order(name));
        seen
    }

    fn cycle_filter(&mut self) {
        self.status_filter = match &self.status_filter {
            None => self.available_statuses.first().cloned(),
            Some(current) => {
                let pos = self.available_statuses.iter().position(|s| s == current);
                match pos {
                    Some(i) if i + 1 < self.available_statuses.len() => {
                        Some(self.available_statuses[i + 1].clone())
                    }
                    _ => None,
                }
            }
        };
        self.apply_filter();
    }

    fn rebuild_statuses(&mut self) {
        let mut statuses: Vec<String> = Vec::new();
        for issue in &self.all_issues {
            if !statuses.contains(&issue.status) {
                statuses.push(issue.status.clone());
            }
        }
        // Sort with a sensible order: active states first, then closed
        let order = |s: &str| match s {
            "In Progress" => 0,
            "Todo" => 1,
            "Backlog" => 2,
            "Triage" => 3,
            "Done" | "Completed" => 4,
            "Canceled" | "Cancelled" => 5,
            _ => 3,
        };
        statuses.sort_by_key(|s| order(s));
        self.available_statuses = statuses;
    }

    fn apply_filter(&mut self) {
        self.filtered = self
            .all_issues
            .iter()
            .filter(|i| {
                self.status_filter
                    .as_ref()
                    .map_or(true, |s| i.status == *s)
                    && self
                        .team_filter
                        .as_ref()
                        .map_or(true, |t| i.team.as_deref() == Some(t.as_str()))
                    && self
                        .project_filter
                        .as_ref()
                        .map_or(true, |p| i.project.as_deref() == Some(p.as_str()))
            })
            .cloned()
            .collect();
        // Reset selection if out of bounds
        if self.filtered.is_empty() {
            self.state.select(Some(0));
        } else if self.selected_index() >= self.filtered.len() {
            self.state.select(Some(0));
        }
    }

    fn set_issues(&mut self, issues: Vec<Issue>) {
        self.all_issues = issues;
        self.rebuild_statuses();
        // If current filter no longer exists in data, reset to All
        if let Some(ref f) = self.status_filter {
            if !self.available_statuses.contains(f) {
                self.status_filter = None;
            }
        }
        // Validate team filter
        if let Some(ref t) = self.team_filter {
            if !self.all_issues.iter().any(|i| i.team.as_deref() == Some(t.as_str())) {
                self.team_filter = None;
                self.project_filter = None;
            }
        }
        // Validate project filter
        if let Some(ref p) = self.project_filter {
            if !self.all_issues.iter().any(|i| i.project.as_deref() == Some(p.as_str())) {
                self.project_filter = None;
            }
        }
        self.apply_filter();
    }

    fn title(&self) -> String {
        match &self.status_filter {
            None => " Issues - All ".to_string(),
            Some(status) => format!(" Issues - {status} "),
        }
    }

    fn bottom_title(&self) -> Option<Line<'_>> {
        match (&self.team_filter, &self.project_filter) {
            (Some(team), Some(project)) => Some(Line::from(vec![
                Span::raw(" "),
                Span::styled(
                    format!("{team} / {project}"),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(" "),
            ])),
            (Some(team), None) => Some(Line::from(vec![
                Span::raw(" "),
                Span::styled(
                    format!("Team: {team}"),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(" "),
            ])),
            _ => None,
        }
    }

    fn format_issue_item<'a>(&self, issue: &'a Issue, width: u16) -> ListItem<'a> {
        let is_claude_active = self
            .active_claude_issue_id
            .as_deref()
            .map(|id| id == issue.id)
            .unwrap_or(false);
        Self::format_issue(issue, is_claude_active, width)
    }

    fn format_issue(issue: &Issue, claude_active: bool, width: u16) -> ListItem<'_> {
        let status_color = match issue.status.as_str() {
            "In Progress" => Color::Yellow,
            "Done" | "Completed" => Color::Green,
            "Canceled" | "Cancelled" => Color::Red,
            "Backlog" => Color::DarkGray,
            _ => Color::White,
        };

        let claude_indicator = if claude_active {
            Span::styled(" ", Style::default().fg(Color::Green))
        } else {
            Span::raw("  ")
        };

        let w = width as usize;

        // Compact layout for narrow panes (< 50 cols)
        if w < 50 {
            let title_max = w.saturating_sub(14); // 2 indicator + 10 id + 2 space
            let line = Line::from(vec![
                claude_indicator,
                Span::styled(
                    format!("{:<8}", truncate(&issue.identifier, 8)),
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw(" "),
                Span::raw(truncate(&issue.title, title_max)),
            ]);
            return ListItem::new(line);
        }

        // Standard layout: adapt title width to fill available space
        // Fixed parts: 2 (indicator) + 10 (id) + 3 (sep) + 3 (sep) + 12 (status) = 30
        // Assignee is variable, budget ~15
        let fixed = 30 + 3 + 15;
        let title_max = w.saturating_sub(fixed).max(10);

        let assignee = issue
            .assignee
            .as_deref()
            .map(|a| format!("@{a}"))
            .unwrap_or_default();

        let line = Line::from(vec![
            claude_indicator,
            Span::styled(
                format!("{:<10}", issue.identifier),
                Style::default().fg(Color::Cyan),
            ),
            Span::raw(" │ "),
            Span::styled(
                format!("{:<width$}", truncate(&issue.title, title_max), width = title_max),
                Style::default(),
            ),
            Span::raw(" │ "),
            Span::styled(
                format!("{:<12}", issue.status),
                Style::default().fg(status_color),
            ),
            Span::raw(" │ "),
            Span::styled(assignee, Style::default().fg(Color::Magenta)),
        ]);

        ListItem::new(line)
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}

impl Component for IssueList {
    fn handle_key_event(&mut self, key: KeyEvent) -> Option<Action> {
        // Ctrl-key combos first
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return match key.code {
                KeyCode::Char('s') => Some(Action::OpenSettings),
                KeyCode::Char('p') => Some(Action::OpenCommandPalette),
                _ => None,
            };
        }

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_down();
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_up();
                None
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(issue) = self.filtered.get(self.selected_index()) {
                    Some(Action::ShowIssueDetail(issue.clone()))
                } else {
                    None
                }
            }
            KeyCode::Char('c') => {
                if let Some(issue) = self.filtered.get(self.selected_index()) {
                    Some(Action::LaunchClaude(issue.id.clone()))
                } else {
                    None
                }
            }
            KeyCode::Char('e') => {
                if let Some(issue) = self.filtered.get(self.selected_index()) {
                    Some(Action::EditIssue(issue.clone()))
                } else {
                    None
                }
            }
            KeyCode::Char('s') => {
                // Quick status cycle
                if let Some(issue) = self.filtered.get(self.selected_index()) {
                    Some(Action::CycleStatus(issue.id.clone()))
                } else {
                    None
                }
            }
            KeyCode::Char('f') => {
                self.cycle_filter();
                None
            }
            KeyCode::Char('T') => {
                if let Some(issue) = self.filtered.get(self.selected_index()) {
                    Some(Action::ViewTranscripts(issue.id.clone()))
                } else {
                    None
                }
            }
            KeyCode::Char('d') => {
                if let Some(issue) = self.filtered.get(self.selected_index()) {
                    Some(Action::ViewDocuments(issue.id.clone()))
                } else {
                    None
                }
            }
            KeyCode::Char('r') => Some(Action::Refresh),
            KeyCode::Char('/') => Some(Action::OpenSearch),
            KeyCode::Char('q') => Some(Action::Quit),
            _ => None,
        }
    }

    fn update(&mut self, action: &Action) -> Option<Action> {
        match action {
            Action::IssuesLoaded(issues) => {
                self.set_issues(issues.clone());
                None
            }
            Action::IssueSaved(updated) => {
                // Update existing or insert new issue
                if let Some(pos) = self.all_issues.iter().position(|i| i.id == updated.id) {
                    self.all_issues[pos] = updated.clone();
                } else {
                    self.all_issues.insert(0, updated.clone());
                }
                self.rebuild_statuses();
                self.apply_filter();
                None
            }
            Action::WorkflowStatesLoaded(states) => {
                self.set_workflow_states(states.clone());
                None
            }
            _ => None,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        // Inner width (minus 2 for borders)
        let inner_width = area.width.saturating_sub(2);
        let items: Vec<ListItem> = if self.filtered.is_empty() {
            let msg = if self.status_filter.is_some() {
                "  No matches. Press 'f' to change filter."
            } else {
                "  No issues. Press 'r' to refresh."
            };
            vec![ListItem::new(Line::from(vec![Span::styled(
                msg,
                Style::default().fg(Color::DarkGray),
            )]))]
        } else {
            self.filtered.iter().map(|i| self.format_issue_item(i, inner_width)).collect()
        };

        let mut block = Block::default()
            .title(self.title())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue));

        if let Some(bottom) = self.bottom_title() {
            block = block.title_bottom(bottom);
        }

        let list = List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(list, area, &mut self.state.clone());
    }
}
