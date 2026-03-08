use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph};

use crate::action::Action;
use crate::config::theme::Theme;
use crate::widgets::modal;

use super::Component;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Select,
    ConfirmCreate,
}

pub struct BranchPicker {
    visible: bool,
    issue_id: String,
    input: String,
    branches: Vec<String>,
    filtered: Vec<usize>,
    selection: usize,
    state: ListState,
    mode: Mode,
    current_branch: Option<String>,
}

impl BranchPicker {
    pub fn new() -> Self {
        Self {
            visible: false,
            issue_id: String::new(),
            input: String::new(),
            branches: Vec::new(),
            filtered: Vec::new(),
            selection: 0,
            state: ListState::default(),
            mode: Mode::Select,
            current_branch: None,
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn show(
        &mut self,
        issue_id: String,
        branches: Vec<String>,
        current_branch: Option<String>,
        identifier_hint: Option<String>,
    ) {
        self.issue_id = issue_id;
        self.branches = branches;
        self.current_branch = current_branch;
        self.input = identifier_hint.unwrap_or_default();
        self.mode = Mode::Select;
        self.refilter();

        // Pre-select current branch if assigned
        if let Some(ref current) = self.current_branch {
            if let Some(pos) = self.filtered.iter().position(|&i| self.branches[i] == *current) {
                self.selection = pos;
            }
        }

        self.sync_list_state();
        self.visible = true;
    }

    fn refilter(&mut self) {
        let query = self.input.to_lowercase();
        self.filtered = self
            .branches
            .iter()
            .enumerate()
            .filter(|(_, b)| query.is_empty() || b.to_lowercase().contains(&query))
            .map(|(i, _)| i)
            .collect();

        self.selection = self.selection.min(self.filtered.len().saturating_sub(1));
        self.sync_list_state();
    }

    fn sync_list_state(&mut self) {
        if self.filtered.is_empty() {
            self.state.select(None);
        } else {
            self.state.select(Some(self.selection));
        }
    }

    fn move_up(&mut self) {
        if self.selection > 0 {
            self.selection -= 1;
            self.sync_list_state();
        }
    }

    fn move_down(&mut self) {
        if self.selection + 1 < self.filtered.len() {
            self.selection += 1;
            self.sync_list_state();
        }
    }

    fn hide(&mut self) {
        self.visible = false;
        self.input.clear();
        self.branches.clear();
        self.filtered.clear();
    }

    fn confirm_selection(&mut self) -> Option<Action> {
        if let Some(&idx) = self.filtered.get(self.selection) {
            let branch = self.branches[idx].clone();
            let issue_id = self.issue_id.clone();
            self.hide();
            Some(Action::SetBranch(issue_id, branch))
        } else if !self.input.trim().is_empty() {
            // No match — offer to create
            self.mode = Mode::ConfirmCreate;
            None
        } else {
            None
        }
    }

    fn confirm_create(&mut self) -> Option<Action> {
        let branch = self.input.trim().to_string();
        if branch.is_empty() {
            return None;
        }
        let issue_id = self.issue_id.clone();
        self.hide();
        Some(Action::CreateAndSetBranch(issue_id, branch))
    }
}

impl Component for BranchPicker {
    fn handle_key_event(&mut self, key: KeyEvent) -> Option<Action> {
        if !self.visible {
            return None;
        }

        match self.mode {
            Mode::ConfirmCreate => match key.code {
                KeyCode::Enter => self.confirm_create(),
                KeyCode::Esc => {
                    self.mode = Mode::Select;
                    None
                }
                _ => None,
            },
            Mode::Select => {
                // Ctrl+x to clear branch assignment
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('x')
                {
                    if self.current_branch.is_some() {
                        let issue_id = self.issue_id.clone();
                        self.hide();
                        return Some(Action::ClearBranch(issue_id));
                    }
                    return None;
                }

                match key.code {
                    KeyCode::Esc => {
                        self.hide();
                        None
                    }
                    KeyCode::Enter => self.confirm_selection(),
                    KeyCode::Up => {
                        self.move_up();
                        None
                    }
                    KeyCode::Down => {
                        self.move_down();
                        None
                    }
                    KeyCode::Backspace => {
                        self.input.pop();
                        self.refilter();
                        None
                    }
                    KeyCode::Char(c) => {
                        // j/k navigate only when input is empty
                        if self.input.is_empty() {
                            match c {
                                'j' => {
                                    self.move_down();
                                    return None;
                                }
                                'k' => {
                                    self.move_up();
                                    return None;
                                }
                                _ => {}
                            }
                        }
                        self.input.push(c);
                        self.refilter();
                        None
                    }
                    _ => None,
                }
            }
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.visible {
            return;
        }

        let s = theme.styles();

        let title = if self.current_branch.is_some() {
            "Select Branch (C-x to clear)"
        } else {
            "Select Branch"
        };

        let inner = modal::render_modal_themed(frame, area, title, 60, 60, Some(&s));

        let chunks = Layout::vertical([
            Constraint::Length(1), // input line
            Constraint::Length(1), // separator
            Constraint::Min(1),   // branch list or confirm
        ])
        .split(inner);

        // Input line
        let input_display = if self.input.is_empty() {
            Span::styled("Type to filter or create...", Style::default().fg(s.muted))
        } else {
            Span::styled(&self.input, Style::default().fg(s.fg))
        };
        let cursor = Span::styled("▏", Style::default().fg(s.accent));
        let input_line = if self.input.is_empty() {
            Line::from(vec![cursor, Span::raw(" "), input_display])
        } else {
            Line::from(vec![input_display, cursor])
        };
        frame.render_widget(Paragraph::new(input_line), chunks[0]);

        // Separator
        let sep = Line::from(Span::styled(
            "─".repeat(chunks[1].width as usize),
            Style::default().fg(s.border),
        ));
        frame.render_widget(Paragraph::new(sep), chunks[1]);

        match self.mode {
            Mode::ConfirmCreate => {
                let msg = vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::raw("Create branch "),
                        Span::styled(
                            self.input.trim(),
                            Style::default().fg(s.accent).add_modifier(Modifier::BOLD),
                        ),
                        Span::raw("?"),
                    ]),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Enter to confirm · Esc to go back",
                        Style::default().fg(s.muted),
                    )),
                ];
                frame.render_widget(Paragraph::new(msg), chunks[2]);
            }
            Mode::Select => {
                if self.filtered.is_empty() {
                    let hint = if self.input.is_empty() {
                        "No branches found"
                    } else {
                        "No matches — press Enter to create this branch"
                    };
                    frame.render_widget(
                        Paragraph::new(Line::from(Span::styled(
                            hint,
                            Style::default().fg(s.muted),
                        ))),
                        chunks[2],
                    );
                } else {
                    let items: Vec<ListItem> = self
                        .filtered
                        .iter()
                        .map(|&idx| {
                            let name = &self.branches[idx];
                            let is_assigned = self
                                .current_branch
                                .as_ref()
                                .map_or(false, |c| c == name);
                            let style = if is_assigned {
                                Style::default().fg(s.accent)
                            } else {
                                Style::default().fg(s.fg)
                            };
                            let prefix = if is_assigned { "● " } else { "  " };
                            ListItem::new(Line::from(Span::styled(
                                format!("{prefix}{name}"),
                                style,
                            )))
                        })
                        .collect();

                    let list = List::new(items)
                        .highlight_style(
                            Style::default()
                                .bg(s.selection)
                                .add_modifier(Modifier::BOLD),
                        )
                        .highlight_symbol("▶ ");

                    frame.render_stateful_widget(list, chunks[2], &mut self.state.clone());
                }
            }
        }
    }
}
