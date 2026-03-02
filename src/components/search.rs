use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph};

use crate::action::Action;
use crate::db::search_repo::SearchResult;
use crate::widgets::modal;

use super::Component;

pub struct SearchOverlay {
    visible: bool,
    query: String,
    cursor: usize,
    results: Vec<SearchResult>,
    state: ListState,
    scope_issue_id: Option<String>,
}

impl SearchOverlay {
    pub fn new() -> Self {
        Self {
            visible: false,
            query: String::new(),
            cursor: 0,
            results: Vec::new(),
            state: ListState::default(),
            scope_issue_id: None,
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn show(&mut self, scope_issue_id: Option<String>) {
        self.visible = true;
        self.query.clear();
        self.cursor = 0;
        self.results.clear();
        self.scope_issue_id = scope_issue_id;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn set_results(&mut self, results: Vec<SearchResult>) {
        self.results = results;
        if !self.results.is_empty() {
            self.state.select(Some(0));
        }
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn scope_issue_id(&self) -> Option<&str> {
        self.scope_issue_id.as_deref()
    }
}

impl Component for SearchOverlay {
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
                let action = if let Some(i) = self.state.selected() {
                    self.results.get(i).map(|result| {
                        Action::StatusMessage(format!(
                            "Selected: {} ({})",
                            result.title, result.source_type
                        ))
                    })
                } else {
                    None
                };
                self.hide();
                action
            }
            KeyCode::Down => {
                let i = self.state.selected().unwrap_or(0);
                if i < self.results.len().saturating_sub(1) {
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
                }
                Some(Action::SearchQuery(self.query.clone()))
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
                Some(Action::SearchQuery(self.query.clone()))
            }
            _ => None,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let inner = modal::render_modal(frame, area, "Search", 60, 50);

        let chunks = Layout::vertical([
            Constraint::Length(1), // Input
            Constraint::Length(1), // Separator
            Constraint::Min(1),   // Results
        ])
        .split(inner);

        // Search input
        let display_query = if self.query.is_empty() {
            "Type to search...".to_string()
        } else {
            self.query.clone()
        };
        let input_style = if self.query.is_empty() {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::White)
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" / ", Style::default().fg(Color::Yellow)),
                Span::styled(display_query, input_style),
            ])),
            chunks[0],
        );

        // Results
        if self.results.is_empty() && !self.query.is_empty() {
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    "  No results",
                    Style::default().fg(Color::DarkGray),
                ))),
                chunks[2],
            );
        } else {
            let items: Vec<ListItem> = self
                .results
                .iter()
                .map(|r| {
                    let type_color = match r.source_type.as_str() {
                        "issue" => Color::Cyan,
                        "document" => Color::Green,
                        "transcript" => Color::Yellow,
                        "summary" => Color::Magenta,
                        _ => Color::White,
                    };

                    ListItem::new(Line::from(vec![
                        Span::styled(
                            format!("[{:<10}] ", r.source_type),
                            Style::default().fg(type_color),
                        ),
                        Span::raw(&r.title),
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
}
