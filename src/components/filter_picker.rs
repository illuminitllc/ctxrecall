use std::collections::{BTreeMap, BTreeSet};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState};

use crate::action::Action;
use crate::tracker::types::{Issue, Team, Project};
use crate::widgets::modal;

use super::Component;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FilterPickerMode {
    Team,
    Project,
}

#[derive(Debug, Clone)]
enum PickerItem {
    Selectable {
        label: String,
        value: Option<String>,  // None = "All" option
        team: Option<String>,   // associated team (project mode)
    },
    Header(String),
}

pub struct FilterPicker {
    visible: bool,
    mode: FilterPickerMode,
    items: Vec<PickerItem>,
    selectable_indices: Vec<usize>,
    selection: usize,
    state: ListState,
}

impl FilterPicker {
    pub fn new() -> Self {
        Self {
            visible: false,
            mode: FilterPickerMode::Team,
            items: Vec::new(),
            selectable_indices: Vec::new(),
            selection: 0,
            state: ListState::default(),
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn show_teams(&mut self, teams: Vec<String>, current: Option<&str>) {
        self.mode = FilterPickerMode::Team;
        self.items.clear();
        self.selectable_indices.clear();

        // "All Teams" option
        self.selectable_indices.push(self.items.len());
        self.items.push(PickerItem::Selectable {
            label: "All Teams".to_string(),
            value: None,
            team: None,
        });

        for team in &teams {
            self.selectable_indices.push(self.items.len());
            self.items.push(PickerItem::Selectable {
                label: team.clone(),
                value: Some(team.clone()),
                team: None,
            });
        }

        // Select current filter or "All"
        self.selection = current
            .and_then(|c| teams.iter().position(|t| t == c).map(|p| p + 1))
            .unwrap_or(0);

        self.sync_list_state();
        self.visible = true;
    }

    pub fn show_projects(
        &mut self,
        issues: &[Issue],
        teams: &[Team],
        projects: &[Project],
        current: Option<&str>,
    ) {
        self.mode = FilterPickerMode::Project;
        self.items.clear();
        self.selectable_indices.clear();

        // Build team ID → name lookup
        let team_name: BTreeMap<&str, &str> = teams
            .iter()
            .map(|t| (t.id.as_str(), t.name.as_str()))
            .collect();

        // Start with projects from the API (complete list)
        let mut team_projects: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for project in projects {
            for tid in &project.team_ids {
                let tname = team_name
                    .get(tid.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| tid.clone());
                team_projects
                    .entry(tname)
                    .or_default()
                    .insert(project.name.clone());
            }
        }

        // Merge in any team/project pairs from issues that weren't in the API data
        for issue in issues {
            if let (Some(team), Some(project)) = (&issue.team, &issue.project) {
                team_projects
                    .entry(team.clone())
                    .or_default()
                    .insert(project.clone());
            }
        }

        // "All Projects" option
        self.selectable_indices.push(self.items.len());
        self.items.push(PickerItem::Selectable {
            label: "All Projects".to_string(),
            value: None,
            team: None,
        });

        let mut current_sel = 0usize;
        for (team, projs) in &team_projects {
            self.items.push(PickerItem::Header(team.clone()));
            for project in projs {
                let idx = self.items.len();
                self.selectable_indices.push(idx);
                if current.map_or(false, |c| c == project) {
                    current_sel = self.selectable_indices.len() - 1;
                }
                self.items.push(PickerItem::Selectable {
                    label: format!("  {project}"),
                    value: Some(project.clone()),
                    team: Some(team.clone()),
                });
            }
        }

        self.selection = current_sel;
        self.sync_list_state();
        self.visible = true;
    }

    fn sync_list_state(&mut self) {
        if let Some(&visual_idx) = self.selectable_indices.get(self.selection) {
            self.state.select(Some(visual_idx));
        }
    }

    fn move_up(&mut self) {
        if self.selection > 0 {
            self.selection -= 1;
            self.sync_list_state();
        }
    }

    fn move_down(&mut self) {
        if self.selection + 1 < self.selectable_indices.len() {
            self.selection += 1;
            self.sync_list_state();
        }
    }

    fn confirm(&mut self) -> Option<Action> {
        let idx = self.selectable_indices.get(self.selection).copied()?;
        let item = self.items.get(idx)?;
        self.visible = false;
        match (&self.mode, item) {
            (FilterPickerMode::Team, PickerItem::Selectable { value, .. }) => {
                Some(Action::SetTeamFilter(value.clone()))
            }
            (FilterPickerMode::Project, PickerItem::Selectable { value, team, .. }) => {
                Some(Action::SetProjectFilter(value.clone(), team.clone()))
            }
            _ => None,
        }
    }

    fn hide(&mut self) {
        self.visible = false;
    }
}

impl Component for FilterPicker {
    fn handle_key_event(&mut self, key: KeyEvent) -> Option<Action> {
        if !self.visible {
            return None;
        }

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.hide();
                None
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_down();
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_up();
                None
            }
            KeyCode::Enter => self.confirm(),
            _ => None,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let title = match self.mode {
            FilterPickerMode::Team => "Filter by Team",
            FilterPickerMode::Project => "Filter by Project",
        };

        let inner = modal::render_modal(frame, area, title, 50, 50);

        if self.items.is_empty() {
            return;
        }

        let items: Vec<ListItem> = self
            .items
            .iter()
            .map(|item| match item {
                PickerItem::Selectable { label, value, .. } => {
                    let style = if value.is_none() {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    };
                    ListItem::new(Line::from(Span::styled(label.as_str(), style)))
                }
                PickerItem::Header(name) => {
                    let header = format!("── {name} ──");
                    ListItem::new(Line::from(Span::styled(
                        header,
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )))
                }
            })
            .collect();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(list, inner, &mut self.state.clone());
    }
}
