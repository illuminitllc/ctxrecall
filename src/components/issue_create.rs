use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};

use crate::action::Action;
use crate::tracker::types::{Issue, IssueStatus, Label, NewIssue, Project, Team};
use crate::widgets::editable_field::{EditFieldAction, EditableField};
use crate::widgets::modal;

use super::Component;

const PRIORITY_LABELS: &[&str] = &["No priority", "Urgent", "High", "Medium", "Low"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CreatePhase {
    SelectTeam,
    SelectProject,
    EditFields,
    SelectLabels,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CreateField {
    Title,
    Description,
    Priority,
    Status,
    Assignee,
    Labels,
}

pub struct IssueCreate {
    visible: bool,
    phase: CreatePhase,

    // Team/project selection
    teams: Vec<(String, String, String)>, // (id, name, key)
    projects: Vec<(String, String)>,      // (id, name) filtered for selected team
    team_state: ListState,
    project_state: ListState,
    selected_team: Option<(String, String, String)>, // (id, name, key)
    selected_project: Option<(String, String)>,      // (id, name)

    // Form fields
    identifier_preview: String,
    title_field: EditableField,
    description_field: EditableField,
    priority: usize,
    status_idx: usize,
    available_statuses: Vec<(String, String)>, // (id, name)
    assignee_idx: usize,                       // 0 = None
    available_assignees: Vec<(String, String)>, // (id, name)
    selected_labels: Vec<(String, String)>,                   // (id, name)
    all_labels: Vec<(String, String, String, Option<String>)>, // (id, name, color, team_id)
    filtered_labels: Vec<(String, String, String)>,            // (id, name, color) for current team

    // Labels sub-modal
    label_state: ListState,
    label_toggles: Vec<bool>,

    focused_field: CreateField,

    // Stored for phase transitions
    projects_all: Vec<Project>,
    workflow_states_all: Vec<IssueStatus>,
    issues_snapshot: Vec<Issue>,
}

impl IssueCreate {
    pub fn new() -> Self {
        Self {
            visible: false,
            phase: CreatePhase::SelectTeam,
            teams: Vec::new(),
            projects: Vec::new(),
            team_state: ListState::default(),
            project_state: ListState::default(),
            selected_team: None,
            selected_project: None,
            identifier_preview: String::new(),
            title_field: EditableField::new("Title", "", false),
            description_field: EditableField::new("Description", "", true),
            priority: 0,
            status_idx: 0,
            available_statuses: Vec::new(),
            assignee_idx: 0,
            available_assignees: Vec::new(),
            selected_labels: Vec::new(),
            all_labels: Vec::new(),
            filtered_labels: Vec::new(),
            label_state: ListState::default(),
            label_toggles: Vec::new(),
            focused_field: CreateField::Title,
            projects_all: Vec::new(),
            workflow_states_all: Vec::new(),
            issues_snapshot: Vec::new(),
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn show(
        &mut self,
        teams: &[Team],
        projects: &[Project],
        issues: &[Issue],
        workflow_states: &[IssueStatus],
        labels: &[Label],
    ) {
        self.teams = teams
            .iter()
            .map(|t| (t.id.clone(), t.name.clone(), t.key.clone()))
            .collect();
        self.all_labels = labels
            .iter()
            .map(|l| (l.id.clone(), l.name.clone(), l.color.clone(), l.team_id.clone()))
            .collect();
        self.available_assignees = derive_assignees(issues);

        // Store projects and workflow states for later phase transitions
        self.projects_all = projects.to_vec();
        self.workflow_states_all = workflow_states.to_vec();
        self.issues_snapshot = issues.to_vec();

        // Reset state
        self.selected_team = None;
        self.selected_project = None;
        self.identifier_preview.clear();
        self.title_field.set_value("");
        self.description_field.set_value("");
        self.priority = 0;
        self.status_idx = 0;
        self.assignee_idx = 0;
        self.selected_labels.clear();
        self.filtered_labels.clear();
        self.label_toggles.clear();
        self.focused_field = CreateField::Title;

        self.team_state = ListState::default();
        if !self.teams.is_empty() {
            self.team_state.select(Some(0));
        }

        self.phase = CreatePhase::SelectTeam;
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    fn on_team_selected(&mut self) {
        let Some(idx) = self.team_state.selected() else {
            return;
        };
        let Some(team) = self.teams.get(idx).cloned() else {
            return;
        };

        // Filter projects for this team
        self.projects = self
            .projects_all
            .iter()
            .filter(|p| p.team_ids.contains(&team.0))
            .map(|p| (p.id.clone(), p.name.clone()))
            .collect();

        // Compute identifier preview
        self.identifier_preview =
            compute_next_identifier(&self.issues_snapshot, &team.2);

        // Populate statuses for this team
        self.available_statuses = self
            .workflow_states_all
            .iter()
            .filter(|s| s.team_id == team.0)
            .map(|s| (s.id.clone(), s.name.clone()))
            .collect();
        self.status_idx = 0;

        self.selected_team = Some(team);

        // Set up project picker
        self.project_state = ListState::default();
        self.project_state.select(Some(0)); // "None" is first
        self.phase = CreatePhase::SelectProject;
    }

    fn on_project_selected(&mut self) {
        let Some(idx) = self.project_state.selected() else {
            return;
        };
        if idx == 0 {
            // "None" selected
            self.selected_project = None;
        } else {
            self.selected_project = self.projects.get(idx - 1).cloned();
        }
        self.focused_field = CreateField::Title;
        self.phase = CreatePhase::EditFields;
    }

    fn build_new_issue(&self) -> Option<NewIssue> {
        let title = self.title_field.value().trim().to_string();
        if title.is_empty() {
            return None;
        }
        let (team_id, _, _) = self.selected_team.as_ref()?;

        let description = {
            let d = self.description_field.value().trim().to_string();
            if d.is_empty() { None } else { Some(d) }
        };

        let project_id = self.selected_project.as_ref().map(|(id, _)| id.clone());

        let priority = if self.priority > 0 {
            Some(self.priority as i32)
        } else {
            None
        };

        let assignee_id = if self.assignee_idx > 0 {
            self.available_assignees
                .get(self.assignee_idx - 1)
                .map(|(id, _)| id.clone())
        } else {
            None
        };

        let label_ids = if self.selected_labels.is_empty() {
            None
        } else {
            Some(
                self.selected_labels
                    .iter()
                    .map(|(id, _)| id.clone())
                    .collect(),
            )
        };

        Some(NewIssue {
            title,
            description,
            team_id: team_id.clone(),
            project_id,
            priority,
            assignee_id,
            label_ids,
        })
    }

    fn next_field(&mut self) {
        self.focused_field = match self.focused_field {
            CreateField::Title => CreateField::Description,
            CreateField::Description => CreateField::Priority,
            CreateField::Priority => CreateField::Status,
            CreateField::Status => CreateField::Assignee,
            CreateField::Assignee => CreateField::Labels,
            CreateField::Labels => CreateField::Title,
        };
    }

    fn prev_field(&mut self) {
        self.focused_field = match self.focused_field {
            CreateField::Title => CreateField::Labels,
            CreateField::Description => CreateField::Title,
            CreateField::Priority => CreateField::Description,
            CreateField::Status => CreateField::Priority,
            CreateField::Assignee => CreateField::Status,
            CreateField::Labels => CreateField::Assignee,
        };
    }

    fn open_label_picker(&mut self) {
        // Filter labels: workspace-wide (team_id=None) + labels for selected team
        let team_id = self.selected_team.as_ref().map(|(id, _, _)| id.as_str());
        self.filtered_labels = self
            .all_labels
            .iter()
            .filter(|(_, _, _, tid)| {
                tid.is_none() || tid.as_deref() == team_id
            })
            .map(|(id, name, color, _)| (id.clone(), name.clone(), color.clone()))
            .collect();

        // Sync toggles from selected_labels
        self.label_toggles = self
            .filtered_labels
            .iter()
            .map(|(id, _, _)| self.selected_labels.iter().any(|(sid, _)| sid == id))
            .collect();
        self.label_state = ListState::default();
        if !self.filtered_labels.is_empty() {
            self.label_state.select(Some(0));
        }
        self.phase = CreatePhase::SelectLabels;
    }

    fn confirm_label_selection(&mut self) {
        self.selected_labels = self
            .filtered_labels
            .iter()
            .zip(self.label_toggles.iter())
            .filter(|(_, toggled)| **toggled)
            .map(|((id, name, _), _)| (id.clone(), name.clone()))
            .collect();
        self.phase = CreatePhase::EditFields;
    }

    fn handle_select_team_key(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.hide();
                None
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let i = self.team_state.selected().unwrap_or(0);
                if i + 1 < self.teams.len() {
                    self.team_state.select(Some(i + 1));
                }
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let i = self.team_state.selected().unwrap_or(0);
                if i > 0 {
                    self.team_state.select(Some(i - 1));
                }
                None
            }
            KeyCode::Enter => {
                self.on_team_selected();
                None
            }
            _ => None,
        }
    }

    fn handle_select_project_key(&mut self, key: KeyEvent) -> Option<Action> {
        let item_count = self.projects.len() + 1; // +1 for "None"
        match key.code {
            KeyCode::Esc => {
                // Go back to team selection
                self.phase = CreatePhase::SelectTeam;
                None
            }
            KeyCode::Char('q') => {
                self.hide();
                None
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let i = self.project_state.selected().unwrap_or(0);
                if i + 1 < item_count {
                    self.project_state.select(Some(i + 1));
                }
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let i = self.project_state.selected().unwrap_or(0);
                if i > 0 {
                    self.project_state.select(Some(i - 1));
                }
                None
            }
            KeyCode::Enter => {
                self.on_project_selected();
                None
            }
            _ => None,
        }
    }

    fn handle_edit_fields_key(&mut self, key: KeyEvent) -> Option<Action> {
        // If a text field is being edited, delegate to it
        if self.title_field.is_editing() {
            match self.title_field.handle_key(key) {
                EditFieldAction::Submit | EditFieldAction::Cancel => {}
                EditFieldAction::None => {}
            }
            return None;
        }
        if self.description_field.is_editing() {
            match self.description_field.handle_key(key) {
                EditFieldAction::Submit | EditFieldAction::Cancel => {}
                EditFieldAction::None => {}
            }
            return None;
        }

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.hide();
                None
            }
            KeyCode::Tab | KeyCode::Char('j') | KeyCode::Down => {
                self.next_field();
                None
            }
            KeyCode::BackTab | KeyCode::Char('k') | KeyCode::Up => {
                self.prev_field();
                None
            }
            KeyCode::Enter => {
                match self.focused_field {
                    CreateField::Title => self.title_field.start_editing(),
                    CreateField::Description => self.description_field.start_editing(),
                    CreateField::Priority => {
                        self.priority = (self.priority + 1) % PRIORITY_LABELS.len();
                    }
                    CreateField::Status => {
                        if !self.available_statuses.is_empty() {
                            self.status_idx =
                                (self.status_idx + 1) % self.available_statuses.len();
                        }
                    }
                    CreateField::Assignee => {
                        let total = self.available_assignees.len() + 1; // +1 for None
                        self.assignee_idx = (self.assignee_idx + 1) % total;
                    }
                    CreateField::Labels => {
                        self.open_label_picker();
                    }
                }
                None
            }
            KeyCode::Char('s') => {
                if let Some(new_issue) = self.build_new_issue() {
                    self.hide();
                    Some(Action::CreateIssue(new_issue))
                } else {
                    Some(Action::Error("Title is required".into()))
                }
            }
            _ => None,
        }
    }

    fn handle_select_labels_key(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Esc => {
                self.phase = CreatePhase::EditFields;
                None
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let i = self.label_state.selected().unwrap_or(0);
                if i + 1 < self.filtered_labels.len() {
                    self.label_state.select(Some(i + 1));
                }
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let i = self.label_state.selected().unwrap_or(0);
                if i > 0 {
                    self.label_state.select(Some(i - 1));
                }
                None
            }
            KeyCode::Char(' ') => {
                if let Some(idx) = self.label_state.selected() {
                    if let Some(toggle) = self.label_toggles.get_mut(idx) {
                        *toggle = !*toggle;
                    }
                }
                None
            }
            KeyCode::Enter => {
                self.confirm_label_selection();
                None
            }
            _ => None,
        }
    }

    fn render_select_team(&self, frame: &mut Frame, area: Rect) {
        let inner = modal::render_modal(frame, area, "New Issue — Select Team", 50, 50);

        if self.teams.is_empty() {
            frame.render_widget(
                Paragraph::new("No teams loaded. Wait for sync to complete."),
                inner,
            );
            return;
        }

        let items: Vec<ListItem> = self
            .teams
            .iter()
            .map(|(_, name, key)| {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{key} "),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::raw(name.as_str()),
                ]))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(list, inner, &mut self.team_state.clone());
    }

    fn render_select_project(&self, frame: &mut Frame, area: Rect) {
        let title = if let Some((_, name, _)) = &self.selected_team {
            format!("New Issue — Select Project ({name})")
        } else {
            "New Issue — Select Project".to_string()
        };
        let inner = modal::render_modal(frame, area, &title, 50, 50);

        let mut items: Vec<ListItem> = vec![ListItem::new(Line::from(Span::styled(
            "None",
            Style::default().fg(Color::Yellow),
        )))];

        for (_, name) in &self.projects {
            items.push(ListItem::new(Line::from(Span::raw(name.as_str()))));
        }

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(list, inner, &mut self.project_state.clone());
    }

    fn render_edit_fields(&self, frame: &mut Frame, area: Rect) {
        let inner = modal::render_modal(frame, area, "New Issue", 70, 70);

        let chunks = Layout::vertical([
            Constraint::Length(1), // Identifier preview
            Constraint::Length(1), // Title
            Constraint::Length(1), // spacer
            Constraint::Min(4),    // Description
            Constraint::Length(1), // spacer
            Constraint::Length(1), // Priority
            Constraint::Length(1), // Status
            Constraint::Length(1), // Assignee
            Constraint::Length(1), // Labels
            Constraint::Length(1), // spacer
            Constraint::Length(1), // Help text
        ])
        .split(inner);

        // Identifier preview
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  Identifier: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    &self.identifier_preview,
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(" (preview)", Style::default().fg(Color::DarkGray)),
            ])),
            chunks[0],
        );

        // Title
        let title_focused = matches!(self.focused_field, CreateField::Title);
        let title_editing = self.title_field.is_editing();
        let marker = if title_focused { "▶ " } else { "  " };
        let title_display = if title_editing {
            insert_cursor(self.title_field.value(), self.title_field.cursor_pos())
        } else if self.title_field.value().is_empty() {
            "(enter title)".to_string()
        } else {
            self.title_field.value().to_string()
        };
        let title_style = if title_editing {
            Style::default().fg(Color::White).bg(Color::DarkGray)
        } else if title_focused {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw(marker),
                Span::styled("Title: ", Style::default().fg(Color::Cyan)),
                Span::styled(title_display, title_style),
            ])),
            chunks[1],
        );

        // Description
        let desc_focused = matches!(self.focused_field, CreateField::Description);
        let desc_editing = self.description_field.is_editing();
        let border_style = if desc_editing {
            Style::default().fg(Color::Yellow)
        } else if desc_focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let desc_marker = if desc_focused { "▶ " } else { "  " };
        let desc_block = Block::default()
            .title(format!("{desc_marker}Description "))
            .borders(Borders::ALL)
            .border_style(border_style);

        let desc_display = if desc_editing {
            insert_cursor(
                self.description_field.value(),
                self.description_field.cursor_pos(),
            )
        } else if self.description_field.value().is_empty() {
            "(optional)".to_string()
        } else {
            self.description_field.value().to_string()
        };
        let desc_style = if desc_editing {
            Style::default().fg(Color::White).bg(Color::DarkGray)
        } else {
            Style::default()
        };
        frame.render_widget(
            Paragraph::new(desc_display)
                .style(desc_style)
                .block(desc_block)
                .wrap(Wrap { trim: false }),
            chunks[3],
        );

        // Priority
        let pri_focused = matches!(self.focused_field, CreateField::Priority);
        let pri_label = PRIORITY_LABELS.get(self.priority).unwrap_or(&"Unknown");
        render_cycle_field(
            frame,
            chunks[5],
            "Priority",
            pri_label,
            pri_focused,
        );

        // Status
        let status_focused = matches!(self.focused_field, CreateField::Status);
        let status_label = self
            .available_statuses
            .get(self.status_idx)
            .map(|(_, n)| n.as_str())
            .unwrap_or("(none)");
        render_cycle_field(frame, chunks[6], "Status", status_label, status_focused);

        // Assignee
        let assignee_focused = matches!(self.focused_field, CreateField::Assignee);
        let assignee_label = if self.assignee_idx == 0 {
            "None"
        } else {
            self.available_assignees
                .get(self.assignee_idx - 1)
                .map(|(_, n)| n.as_str())
                .unwrap_or("None")
        };
        render_cycle_field(
            frame,
            chunks[7],
            "Assignee",
            assignee_label,
            assignee_focused,
        );

        // Labels
        let labels_focused = matches!(self.focused_field, CreateField::Labels);
        let label_marker = if labels_focused { "▶ " } else { "  " };
        let labels_display = if self.selected_labels.is_empty() {
            "None".to_string()
        } else {
            self.selected_labels
                .iter()
                .map(|(_, n)| n.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        };
        let labels_style = if labels_focused {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw(label_marker),
                Span::styled("Labels: ", Style::default().fg(Color::Cyan)),
                Span::styled(labels_display, labels_style),
                if labels_focused {
                    Span::styled(" (Enter to pick)", Style::default().fg(Color::DarkGray))
                } else {
                    Span::raw("")
                },
            ])),
            chunks[8],
        );

        // Help text
        let help_text = if self.title_field.is_editing() || self.description_field.is_editing() {
            " Type to edit | Esc: stop editing | Enter: confirm"
        } else {
            " Tab/j/k: navigate | Enter: edit/cycle | s: save | Esc: cancel"
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                help_text,
                Style::default().fg(Color::DarkGray),
            ))),
            chunks[10],
        );
    }

    fn render_select_labels(&self, frame: &mut Frame, area: Rect) {
        // First render the edit fields behind
        self.render_edit_fields(frame, area);

        // Then overlay the label picker
        let inner = modal::render_modal(frame, area, "Select Labels", 40, 50);

        if self.filtered_labels.is_empty() {
            frame.render_widget(
                Paragraph::new("No labels available for this team."),
                inner,
            );
            return;
        }

        let items: Vec<ListItem> = self
            .filtered_labels
            .iter()
            .enumerate()
            .map(|(i, (_, name, color))| {
                let toggled = self.label_toggles.get(i).copied().unwrap_or(false);
                let check = if toggled { "[x]" } else { "[ ]" };
                let dot_color = parse_hex_color(color);
                ListItem::new(Line::from(vec![
                    Span::raw(format!("{check} ")),
                    Span::styled("● ", Style::default().fg(dot_color)),
                    Span::raw(name.as_str()),
                ]))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        // Help at bottom
        let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(inner);

        frame.render_stateful_widget(list, chunks[0], &mut self.label_state.clone());
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                " Space: toggle | Enter: confirm | Esc: cancel",
                Style::default().fg(Color::DarkGray),
            ))),
            chunks[1],
        );
    }
}

impl Component for IssueCreate {
    fn handle_key_event(&mut self, key: KeyEvent) -> Option<Action> {
        if !self.visible {
            return None;
        }

        match self.phase {
            CreatePhase::SelectTeam => self.handle_select_team_key(key),
            CreatePhase::SelectProject => self.handle_select_project_key(key),
            CreatePhase::EditFields => self.handle_edit_fields_key(key),
            CreatePhase::SelectLabels => self.handle_select_labels_key(key),
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        match self.phase {
            CreatePhase::SelectTeam => self.render_select_team(frame, area),
            CreatePhase::SelectProject => self.render_select_project(frame, area),
            CreatePhase::EditFields => self.render_edit_fields(frame, area),
            CreatePhase::SelectLabels => self.render_select_labels(frame, area),
        }
    }
}

fn compute_next_identifier(issues: &[Issue], team_key: &str) -> String {
    let max_num = issues
        .iter()
        .filter(|i| i.identifier.starts_with(team_key))
        .filter_map(|i| {
            i.identifier
                .strip_prefix(team_key)
                .and_then(|s| s.strip_prefix('-'))
                .and_then(|n| n.parse::<u32>().ok())
        })
        .max()
        .unwrap_or(0);
    format!("{team_key}-{}", max_num + 1)
}

fn derive_assignees(issues: &[Issue]) -> Vec<(String, String)> {
    let mut seen = Vec::new();
    for issue in issues {
        if let (Some(id), Some(name)) = (&issue.assignee_id, &issue.assignee) {
            if !seen.iter().any(|(sid, _): &(String, String)| sid == id) {
                seen.push((id.clone(), name.clone()));
            }
        }
    }
    seen.sort_by(|a, b| a.1.cmp(&b.1));
    seen
}

fn insert_cursor(text: &str, cursor: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    let before: String = chars[..cursor].iter().collect();
    let after: String = chars[cursor..].iter().collect();
    format!("{before}│{after}")
}

fn render_cycle_field(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    focused: bool,
) {
    let marker = if focused { "▶ " } else { "  " };
    let val_style = if focused {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw(marker),
            Span::styled(format!("{label}: "), Style::default().fg(Color::Cyan)),
            Span::styled(value, val_style),
            if focused {
                Span::styled(" (Enter to cycle)", Style::default().fg(Color::DarkGray))
            } else {
                Span::raw("")
            },
        ])),
        area,
    );
}

fn parse_hex_color(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');
    if hex.len() >= 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(128);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(128);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(128);
        Color::Rgb(r, g, b)
    } else {
        Color::White
    }
}
