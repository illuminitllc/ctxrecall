use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph};

use crate::action::Action;
use crate::widgets::modal;

use super::Component;

struct Command {
    name: &'static str,
    description: &'static str,
    action: Action,
    hotkey: &'static str,
}

pub struct CommandPalette {
    visible: bool,
    query: String,
    cursor: usize,
    commands: Vec<Command>,
    filtered_indices: Vec<usize>,
    state: ListState,
}

impl CommandPalette {
    pub fn new() -> Self {
        let commands = vec![
            Command {
                name: "Refresh Issues",
                description: "Sync issues from Linear",
                action: Action::Refresh,
                hotkey: "r",
            },
            Command {
                name: "Search",
                description: "Full-text search across issues and documents",
                action: Action::OpenSearch,
                hotkey: "/",
            },
            Command {
                name: "Settings",
                description: "Open settings panel",
                action: Action::OpenSettings,
                hotkey: "C-s",
            },
            Command {
                name: "Quit",
                description: "Exit ctxrecall",
                action: Action::Quit,
                hotkey: "q",
            },
        ];

        let filtered_indices: Vec<usize> = (0..commands.len()).collect();
        let mut state = ListState::default();
        if !commands.is_empty() {
            state.select(Some(0));
        }

        Self {
            visible: false,
            query: String::new(),
            cursor: 0,
            commands,
            filtered_indices,
            state,
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn show(&mut self) {
        self.visible = true;
        self.query.clear();
        self.cursor = 0;
        self.filter();
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    fn filter(&mut self) {
        if self.query.is_empty() {
            self.filtered_indices = (0..self.commands.len()).collect();
        } else {
            let q = self.query.to_lowercase();
            self.filtered_indices = self
                .commands
                .iter()
                .enumerate()
                .filter(|(_, cmd)| {
                    cmd.name.to_lowercase().contains(&q)
                        || cmd.description.to_lowercase().contains(&q)
                })
                .map(|(i, _)| i)
                .collect();
        }

        if !self.filtered_indices.is_empty() {
            self.state.select(Some(0));
        } else {
            self.state.select(None);
        }
    }
}

impl Component for CommandPalette {
    fn handle_key_event(&mut self, key: KeyEvent) -> Option<Action> {
        if !self.visible {
            return None;
        }

        match key.code {
            KeyCode::Esc => {
                self.hide();
                None
            }
            KeyCode::Enter => {
                let action = self.state.selected().and_then(|i| {
                    self.filtered_indices
                        .get(i)
                        .map(|&idx| self.commands[idx].action.clone())
                });
                self.hide();
                action
            }
            KeyCode::Down => {
                let i = self.state.selected().unwrap_or(0);
                if i < self.filtered_indices.len().saturating_sub(1) {
                    self.state.select(Some(i + 1));
                }
                None
            }
            KeyCode::Up => {
                let i = self.state.selected().unwrap_or(0);
                if i > 0 {
                    self.state.select(Some(i - 1));
                }
                None
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    let byte_idx = self
                        .query
                        .char_indices()
                        .nth(self.cursor - 1)
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    let next = self
                        .query
                        .char_indices()
                        .nth(self.cursor)
                        .map(|(i, _)| i)
                        .unwrap_or(self.query.len());
                    self.query.replace_range(byte_idx..next, "");
                    self.cursor -= 1;
                    self.filter();
                }
                None
            }
            KeyCode::Char(c) => {
                let byte_idx = self
                    .query
                    .char_indices()
                    .nth(self.cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(self.query.len());
                self.query.insert(byte_idx, c);
                self.cursor += 1;
                self.filter();
                None
            }
            _ => None,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let inner = modal::render_modal(frame, area, "Command Palette", 50, 40);

        let chunks = Layout::vertical([
            Constraint::Length(1), // Input
            Constraint::Length(1), // Separator
            Constraint::Min(1),   // Commands
        ])
        .split(inner);

        // Input
        let display = if self.query.is_empty() {
            "Type a command...".to_string()
        } else {
            self.query.clone()
        };
        let style = if self.query.is_empty() {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::White)
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" > ", Style::default().fg(Color::Yellow)),
                Span::styled(display, style),
            ])),
            chunks[0],
        );

        // Commands
        let items: Vec<ListItem> = self
            .filtered_indices
            .iter()
            .map(|&i| {
                let cmd = &self.commands[i];
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{:<20}", cmd.name),
                        Style::default().fg(Color::White),
                    ),
                    Span::styled(cmd.description, Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("  {}", cmd.hotkey),
                        Style::default().fg(Color::Yellow),
                    ),
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

        frame.render_stateful_widget(list, chunks[2], &mut self.state.clone());
    }
}
