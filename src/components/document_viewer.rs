use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};

use crate::action::Action;
use crate::db::document_repo::Document;
use crate::widgets::modal;

use super::Component;

const DOC_TYPES: &[&str] = &["plan", "prd", "tasks", "memories", "notes"];

enum Mode {
    List,
    View,
}

pub struct DocumentViewer {
    visible: bool,
    issue_id: Option<String>,
    documents: Vec<Document>,
    list_state: ListState,
    mode: Mode,
    content: String,
    scroll: u16,
    current_doc: Option<Document>,
}

impl DocumentViewer {
    pub fn new() -> Self {
        Self {
            visible: false,
            issue_id: None,
            documents: Vec::new(),
            list_state: ListState::default(),
            mode: Mode::List,
            content: String::new(),
            scroll: 0,
            current_doc: None,
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn show(&mut self, issue_id: &str, documents: Vec<Document>) {
        self.issue_id = Some(issue_id.to_string());
        self.documents = documents;
        self.mode = Mode::List;
        self.scroll = 0;
        self.visible = true;
        if !self.documents.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    fn view_document(&mut self, index: usize) {
        if let Some(doc) = self.documents.get(index) {
            self.content = doc.content.clone();
            self.current_doc = Some(doc.clone());
            self.scroll = 0;
            self.mode = Mode::View;
        }
    }
}

impl Component for DocumentViewer {
    fn handle_key_event(&mut self, key: KeyEvent) -> Option<Action> {
        if !self.visible {
            return None;
        }

        match self.mode {
            Mode::List => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.hide();
                    None
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    let i = self.list_state.selected().unwrap_or(0);
                    if i < self.documents.len().saturating_sub(1) {
                        self.list_state.select(Some(i + 1));
                    }
                    None
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    let i = self.list_state.selected().unwrap_or(0);
                    if i > 0 {
                        self.list_state.select(Some(i - 1));
                    }
                    None
                }
                KeyCode::Enter => {
                    if let Some(i) = self.list_state.selected() {
                        self.view_document(i);
                    }
                    None
                }
                KeyCode::Char('n') => {
                    // Create new document - emit action
                    if let Some(issue_id) = &self.issue_id {
                        Some(Action::StatusMessage(format!(
                            "Create document for {issue_id} (not yet implemented)"
                        )))
                    } else {
                        None
                    }
                }
                _ => None,
            },
            Mode::View => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.mode = Mode::List;
                    None
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    self.scroll = self.scroll.saturating_add(1);
                    None
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.scroll = self.scroll.saturating_sub(1);
                    None
                }
                _ => None,
            },
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let inner = modal::render_modal(frame, area, "Documents", 70, 70);

        match self.mode {
            Mode::List => {
                let chunks = Layout::vertical([
                    Constraint::Min(1),
                    Constraint::Length(1),
                ])
                .split(inner);

                if self.documents.is_empty() {
                    frame.render_widget(
                        Paragraph::new(Line::from(Span::styled(
                            "  No documents. Press 'n' to create one.",
                            Style::default().fg(Color::DarkGray),
                        ))),
                        chunks[0],
                    );
                } else {
                    let items: Vec<ListItem> = self
                        .documents
                        .iter()
                        .map(|d| {
                            let type_color = match d.doc_type.as_str() {
                                "plan" => Color::Cyan,
                                "prd" => Color::Green,
                                "tasks" => Color::Yellow,
                                "memories" => Color::Magenta,
                                "notes" => Color::White,
                                _ => Color::DarkGray,
                            };

                            ListItem::new(Line::from(vec![
                                Span::styled(
                                    format!("[{:<8}] ", d.doc_type),
                                    Style::default().fg(type_color),
                                ),
                                Span::raw(&d.title),
                                Span::styled(
                                    format!("  ({})", d.updated_at),
                                    Style::default().fg(Color::DarkGray),
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

                    frame.render_stateful_widget(list, chunks[0], &mut self.list_state.clone());
                }

                frame.render_widget(
                    Paragraph::new(Line::from(Span::styled(
                        " j/k: navigate | Enter: view | n: new | Esc: close",
                        Style::default().fg(Color::DarkGray),
                    ))),
                    chunks[1],
                );
            }
            Mode::View => {
                let chunks = Layout::vertical([
                    Constraint::Length(1),
                    Constraint::Min(1),
                    Constraint::Length(1),
                ])
                .split(inner);

                // Title
                let title = self
                    .current_doc
                    .as_ref()
                    .map(|d| format!(" {} [{}]", d.title, d.doc_type))
                    .unwrap_or_default();

                frame.render_widget(
                    Paragraph::new(Line::from(Span::styled(
                        title,
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ))),
                    chunks[0],
                );

                // Content
                frame.render_widget(
                    Paragraph::new(Text::raw(&self.content))
                        .wrap(Wrap { trim: false })
                        .scroll((self.scroll, 0))
                        .block(
                            Block::default()
                                .borders(Borders::TOP)
                                .border_style(Style::default().fg(Color::DarkGray)),
                        ),
                    chunks[1],
                );

                frame.render_widget(
                    Paragraph::new(Line::from(Span::styled(
                        " j/k: scroll | Esc: back to list",
                        Style::default().fg(Color::DarkGray),
                    ))),
                    chunks[2],
                );
            }
        }
    }
}
