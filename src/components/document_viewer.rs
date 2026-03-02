use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};

use crate::action::Action;
use crate::db::document_repo::Document;
use crate::widgets::editable_field::{EditFieldAction, EditableField};
use crate::widgets::modal;

use super::Component;

const DOC_TYPES: &[&str] = &["plan", "prd", "tasks", "memories", "notes", "custom..."];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    List,
    View,
    Create,
    Edit,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CreatePhase {
    SelectType,
    EditTitle,
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

    // Create mode
    doc_type_index: usize,
    title_field: EditableField,
    custom_type_field: EditableField,
    create_phase: CreatePhase,
    is_custom_type: bool,

    // Edit mode
    content_field: EditableField,
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

            doc_type_index: 0,
            title_field: EditableField::new("Title", "", false),
            custom_type_field: EditableField::new("Custom type", "", false),
            create_phase: CreatePhase::SelectType,
            is_custom_type: false,

            content_field: EditableField::new("Content", "", true),
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

    fn enter_create_mode(&mut self) {
        self.doc_type_index = 0;
        self.is_custom_type = false;
        self.title_field.set_value("");
        self.custom_type_field.set_value("");
        self.create_phase = CreatePhase::SelectType;
        self.mode = Mode::Create;
    }

    fn enter_edit_mode(&mut self) {
        if let Some(doc) = &self.current_doc {
            self.content_field.set_value(&doc.content);
            self.content_field.start_editing();
            self.mode = Mode::Edit;
        }
    }

    fn selected_doc_type(&self) -> String {
        if self.is_custom_type {
            let v = self.custom_type_field.value().trim().to_string();
            if v.is_empty() { "custom".to_string() } else { v }
        } else {
            DOC_TYPES[self.doc_type_index].to_string()
        }
    }

    fn handle_list_key(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
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
                if self.issue_id.is_some() {
                    self.enter_create_mode();
                }
                None
            }
            _ => None,
        }
    }

    fn handle_view_key(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
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
            KeyCode::Char('e') => {
                self.enter_edit_mode();
                None
            }
            _ => None,
        }
    }

    fn handle_create_key(&mut self, key: KeyEvent) -> Option<Action> {
        match self.create_phase {
            CreatePhase::SelectType => {
                // If editing custom type field, delegate
                if self.custom_type_field.is_editing() {
                    match self.custom_type_field.handle_key(key) {
                        EditFieldAction::Submit => {
                            // Advance to title
                            self.create_phase = CreatePhase::EditTitle;
                            self.title_field.start_editing();
                        }
                        EditFieldAction::Cancel => {
                            self.is_custom_type = false;
                            self.doc_type_index = 0;
                        }
                        EditFieldAction::None => {}
                    }
                    return None;
                }

                match key.code {
                    KeyCode::Esc => {
                        self.mode = Mode::List;
                    }
                    KeyCode::Enter => {
                        if DOC_TYPES[self.doc_type_index] == "custom..." {
                            self.is_custom_type = true;
                            self.custom_type_field.set_value("");
                            self.custom_type_field.start_editing();
                        } else {
                            self.is_custom_type = false;
                            // Advance to title
                            self.create_phase = CreatePhase::EditTitle;
                            self.title_field.start_editing();
                        }
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        self.doc_type_index = (self.doc_type_index + 1) % DOC_TYPES.len();
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if self.doc_type_index == 0 {
                            self.doc_type_index = DOC_TYPES.len() - 1;
                        } else {
                            self.doc_type_index -= 1;
                        }
                    }
                    KeyCode::Tab => {
                        // Quick advance to title with current type
                        if DOC_TYPES[self.doc_type_index] == "custom..." {
                            self.is_custom_type = true;
                            self.custom_type_field.set_value("");
                            self.custom_type_field.start_editing();
                        } else {
                            self.is_custom_type = false;
                            self.create_phase = CreatePhase::EditTitle;
                            self.title_field.start_editing();
                        }
                    }
                    _ => {}
                }
                None
            }
            CreatePhase::EditTitle => {
                if self.title_field.is_editing() {
                    match self.title_field.handle_key(key) {
                        EditFieldAction::Submit => {
                            // Create the document
                            let title = self.title_field.value().trim().to_string();
                            if title.is_empty() {
                                return Some(Action::Error("Title is required".into()));
                            }
                            let doc_type = self.selected_doc_type();
                            let Some(issue_id) = self.issue_id.clone() else {
                                return Some(Action::Error("No issue selected".into()));
                            };
                            return Some(Action::CreateDocument { issue_id, doc_type, title });
                        }
                        EditFieldAction::Cancel => {
                            self.create_phase = CreatePhase::SelectType;
                        }
                        EditFieldAction::None => {}
                    }
                    None
                } else {
                    match key.code {
                        KeyCode::Esc => {
                            self.create_phase = CreatePhase::SelectType;
                            None
                        }
                        KeyCode::Enter => {
                            self.title_field.start_editing();
                            None
                        }
                        KeyCode::BackTab => {
                            self.create_phase = CreatePhase::SelectType;
                            None
                        }
                        _ => None,
                    }
                }
            }
        }
    }

    fn handle_edit_key(&mut self, key: KeyEvent) -> Option<Action> {
        if self.content_field.is_editing() {
            match self.content_field.handle_key(key) {
                EditFieldAction::Cancel => {
                    self.mode = Mode::View;
                }
                EditFieldAction::Submit => {
                    // Save content
                    if let Some(doc) = &self.current_doc {
                        let content = self.content_field.value().to_string();
                        let doc_id = doc.id.clone();
                        self.content = content.clone();
                        self.mode = Mode::View;
                        return Some(Action::SaveDocumentContent { doc_id, content });
                    }
                    self.mode = Mode::View;
                }
                EditFieldAction::None => {}
            }
            return None;
        }
        // Not editing — shouldn't happen, but handle gracefully
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::View;
            }
            KeyCode::Enter | KeyCode::Char('e') => {
                self.content_field.start_editing();
            }
            _ => {}
        }
        None
    }

    fn render_list(&self, frame: &mut Frame, inner: Rect) {
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
                        _ => Color::LightBlue,
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

    fn render_view(&self, frame: &mut Frame, inner: Rect) {
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
                " j/k: scroll | e: edit | Esc: back to list",
                Style::default().fg(Color::DarkGray),
            ))),
            chunks[2],
        );
    }

    fn render_create(&self, frame: &mut Frame, inner: Rect) {
        let chunks = Layout::vertical([
            Constraint::Length(1), // Type field
            Constraint::Length(1), // Custom type (if applicable)
            Constraint::Length(1), // Title field
            Constraint::Min(1),   // spacer
            Constraint::Length(1), // Help text
        ])
        .split(inner);

        // Type selector
        let type_focused = self.create_phase == CreatePhase::SelectType && !self.custom_type_field.is_editing();
        let type_marker = if type_focused { "▶ " } else { "  " };
        let type_val = DOC_TYPES[self.doc_type_index];
        let type_style = if type_focused {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw(type_marker),
                Span::styled("Type: ", Style::default().fg(Color::Cyan)),
                Span::styled(type_val, type_style),
                if type_focused {
                    Span::styled(" (j/k to cycle, Enter to select)", Style::default().fg(Color::DarkGray))
                } else {
                    Span::raw("")
                },
            ])),
            chunks[0],
        );

        // Custom type field (only when custom is selected)
        if self.is_custom_type {
            let editing = self.custom_type_field.is_editing();
            let display = if editing {
                insert_cursor(self.custom_type_field.value(), self.custom_type_field.cursor_pos())
            } else if self.custom_type_field.value().is_empty() {
                "(enter type name)".to_string()
            } else {
                self.custom_type_field.value().to_string()
            };
            let style = if editing {
                Style::default().fg(Color::White).bg(Color::DarkGray)
            } else {
                Style::default()
            };
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Custom: ", Style::default().fg(Color::Cyan)),
                    Span::styled(display, style),
                ])),
                chunks[1],
            );
        }

        // Title field
        let title_focused = self.create_phase == CreatePhase::EditTitle;
        let title_editing = self.title_field.is_editing();
        let title_marker = if title_focused { "▶ " } else { "  " };
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
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw(title_marker),
                Span::styled("Title: ", Style::default().fg(Color::Cyan)),
                Span::styled(title_display, title_style),
            ])),
            chunks[2],
        );

        // Help text
        let help = if self.custom_type_field.is_editing() {
            " Type to edit | Enter: confirm | Esc: cancel"
        } else if title_editing {
            " Type to edit | Enter: create | Esc: back"
        } else {
            " j/k: cycle type | Tab/Enter: next | Esc: cancel"
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                help,
                Style::default().fg(Color::DarkGray),
            ))),
            chunks[4],
        );
    }

    fn render_edit(&self, frame: &mut Frame, inner: Rect) {
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
            .map(|d| format!(" Editing: {} [{}]", d.title, d.doc_type))
            .unwrap_or_default();

        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                title,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ))),
            chunks[0],
        );

        // Editable content area
        let display = if self.content_field.is_editing() {
            insert_cursor(self.content_field.value(), self.content_field.cursor_pos())
        } else {
            self.content_field.value().to_string()
        };
        let style = if self.content_field.is_editing() {
            Style::default().fg(Color::White).bg(Color::DarkGray)
        } else {
            Style::default()
        };
        frame.render_widget(
            Paragraph::new(Text::raw(&display))
                .style(style)
                .wrap(Wrap { trim: false })
                .block(
                    Block::default()
                        .borders(Borders::TOP)
                        .border_style(Style::default().fg(Color::Yellow)),
                ),
            chunks[1],
        );

        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                " Shift+Enter: newline | Enter: save | Esc: cancel",
                Style::default().fg(Color::DarkGray),
            ))),
            chunks[2],
        );
    }
}

impl Component for DocumentViewer {
    fn handle_key_event(&mut self, key: KeyEvent) -> Option<Action> {
        if !self.visible {
            return None;
        }

        match self.mode {
            Mode::List => self.handle_list_key(key),
            Mode::View => self.handle_view_key(key),
            Mode::Create => self.handle_create_key(key),
            Mode::Edit => self.handle_edit_key(key),
        }
    }

    fn update(&mut self, action: &Action) -> Option<Action> {
        if let Action::DocumentCreated(doc) = action {
            if self.visible && self.issue_id.as_deref() == Some(&doc.issue_id) {
                self.documents.push(doc.clone());
                self.list_state.select(Some(self.documents.len() - 1));
                self.mode = Mode::List;
            }
        }
        None
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let modal_title = match self.mode {
            Mode::Create => "New Document",
            _ => "Documents",
        };
        let inner = modal::render_modal(frame, area, modal_title, 70, 70);

        match self.mode {
            Mode::List => self.render_list(frame, inner),
            Mode::View => self.render_view(frame, inner),
            Mode::Create => self.render_create(frame, inner),
            Mode::Edit => self.render_edit(frame, inner),
        }
    }
}

fn insert_cursor(text: &str, cursor: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    let cursor = cursor.min(chars.len());
    let before: String = chars[..cursor].iter().collect();
    let after: String = chars[cursor..].iter().collect();
    format!("{before}│{after}")
}
