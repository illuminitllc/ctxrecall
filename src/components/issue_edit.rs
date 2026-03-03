use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::action::Action;
use crate::config::theme::Theme;
use crate::tracker::types::{Issue, IssueUpdate};
use crate::widgets::editable_field::{EditFieldAction, EditableField};
use crate::widgets::modal;

use super::Component;

const PRIORITY_LABELS: &[&str] = &["No priority", "Urgent", "High", "Medium", "Low"];

enum EditField {
    Title,
    Description,
    Priority,
}

pub struct IssueEdit {
    visible: bool,
    issue: Option<Issue>,
    title_field: EditableField,
    description_field: EditableField,
    priority: usize,
    focused_field: EditField,
}

impl IssueEdit {
    pub fn new() -> Self {
        Self {
            visible: false,
            issue: None,
            title_field: EditableField::new("Title", "", false),
            description_field: EditableField::new("Description", "", true),
            priority: 0,
            focused_field: EditField::Title,
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn show(&mut self, issue: Issue) {
        self.title_field.set_value(&issue.title);
        self.description_field
            .set_value(issue.description.as_deref().unwrap_or(""));
        self.priority = issue.priority as usize;
        self.focused_field = EditField::Title;
        self.issue = Some(issue);
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn set_description_value(&mut self, value: &str) {
        self.description_field.set_value(value);
    }

    fn build_update(&self) -> Option<(String, IssueUpdate)> {
        let issue = self.issue.as_ref()?;
        let mut update = IssueUpdate::default();
        let mut changed = false;

        if self.title_field.value() != issue.title {
            update.title = Some(self.title_field.value().to_string());
            changed = true;
        }
        let current_desc = issue.description.as_deref().unwrap_or("");
        if self.description_field.value() != current_desc {
            update.description = Some(self.description_field.value().to_string());
            changed = true;
        }
        if self.priority != issue.priority as usize {
            update.priority = Some(self.priority as i32);
            changed = true;
        }

        if changed {
            Some((issue.id.clone(), update))
        } else {
            None
        }
    }

    fn cycle_priority(&mut self) {
        self.priority = (self.priority + 1) % PRIORITY_LABELS.len();
    }
}

impl Component for IssueEdit {
    fn handle_key_event(&mut self, key: KeyEvent) -> Option<Action> {
        if !self.visible {
            return None;
        }

        // If a field is being edited, delegate to it
        if self.title_field.is_editing() {
            match self.title_field.handle_key(key) {
                EditFieldAction::Submit | EditFieldAction::Cancel | EditFieldAction::OpenExternal => {}
                EditFieldAction::None => {}
            }
            return None;
        }

        if self.description_field.is_editing() {
            match self.description_field.handle_key(key) {
                EditFieldAction::Submit | EditFieldAction::Cancel => {}
                EditFieldAction::OpenExternal => {
                    self.description_field.stop_editing();
                    return Some(Action::OpenExternalEditor {
                        field_id: "description".into(),
                        current_value: self.description_field.value().to_string(),
                    });
                }
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
                self.focused_field = match self.focused_field {
                    EditField::Title => EditField::Description,
                    EditField::Description => EditField::Priority,
                    EditField::Priority => EditField::Title,
                };
                None
            }
            KeyCode::BackTab | KeyCode::Char('k') | KeyCode::Up => {
                self.focused_field = match self.focused_field {
                    EditField::Title => EditField::Priority,
                    EditField::Description => EditField::Title,
                    EditField::Priority => EditField::Description,
                };
                None
            }
            KeyCode::Enter => {
                match self.focused_field {
                    EditField::Title => self.title_field.start_editing(),
                    EditField::Description => self.description_field.start_editing(),
                    EditField::Priority => self.cycle_priority(),
                }
                None
            }
            KeyCode::Char('s') => {
                // Save
                if let Some((id, update)) = self.build_update() {
                    self.hide();
                    Some(Action::SaveIssueUpdate(id, update))
                } else {
                    self.hide();
                    Some(Action::StatusMessage("No changes to save".into()))
                }
            }
            _ => None,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.visible {
            return;
        }

        let s = theme.styles();
        let inner = modal::render_modal_themed(frame, area, "Edit Issue", 70, 60, Some(&s));

        let chunks = Layout::vertical([
            Constraint::Length(1), // Title
            Constraint::Length(1), // spacer
            Constraint::Min(5),   // Description
            Constraint::Length(1), // spacer
            Constraint::Length(1), // Priority
            Constraint::Length(1), // spacer
            Constraint::Length(1), // Help line
        ])
        .split(inner);

        // Title
        let title_focused = matches!(self.focused_field, EditField::Title);
        let title_editing = self.title_field.is_editing();
        let focus_marker = if title_focused { "▶ " } else { "  " };

        let title_display = if title_editing {
            insert_cursor(self.title_field.value(), self.title_field.cursor_pos())
        } else {
            self.title_field.value().to_string()
        };
        let title_style = if title_editing {
            Style::default().fg(s.fg).bg(s.selection)
        } else if title_focused {
            Style::default().fg(s.warning).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw(focus_marker),
                Span::styled("Title: ", Style::default().fg(s.accent)),
                Span::styled(title_display, title_style),
            ])),
            chunks[0],
        );

        // Description — rendered in a bordered block with word wrap
        let desc_focused = matches!(self.focused_field, EditField::Description);
        let desc_editing = self.description_field.is_editing();

        let border_style = if desc_editing {
            Style::default().fg(s.warning)
        } else if desc_focused {
            Style::default().fg(s.accent)
        } else {
            Style::default().fg(s.muted)
        };
        let marker = if desc_focused { "▶ " } else { "  " };
        let desc_block = Block::default()
            .title(format!("{marker}Description "))
            .borders(Borders::ALL)
            .border_style(border_style);

        let desc_display = if desc_editing {
            insert_cursor(
                self.description_field.value(),
                self.description_field.cursor_pos(),
            )
        } else if self.description_field.value().is_empty() {
            "(empty)".to_string()
        } else {
            self.description_field.value().to_string()
        };
        let desc_style = if desc_editing {
            Style::default().fg(s.fg).bg(s.selection)
        } else {
            Style::default()
        };
        frame.render_widget(
            Paragraph::new(desc_display)
                .style(desc_style)
                .block(desc_block)
                .wrap(Wrap { trim: false }),
            chunks[2],
        );

        // Priority
        let pri_label = PRIORITY_LABELS.get(self.priority).unwrap_or(&"Unknown");
        let pri_focused = matches!(self.focused_field, EditField::Priority);
        let pri_style = if pri_focused {
            Style::default().fg(s.warning).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let focus_marker = if pri_focused { "▶ " } else { "  " };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw(focus_marker),
                Span::styled("Priority: ", Style::default().fg(s.accent)),
                Span::styled(*pri_label, pri_style),
                Span::styled(" (Enter to cycle)", Style::default().fg(s.muted)),
            ])),
            chunks[4],
        );

        // Help — context-sensitive
        let help_text = if desc_editing {
            " Type to edit | Enter: new line | Esc: stop editing | C-v: $EDITOR"
        } else if title_editing {
            " Type to edit | Enter: confirm | Esc: cancel"
        } else {
            " Tab/j/k: navigate | Enter: edit field | s: save | Esc: cancel"
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                help_text,
                Style::default().fg(s.muted),
            ))),
            chunks[6],
        );
    }
}

/// Insert a visible cursor character at the given char position.
fn insert_cursor(text: &str, cursor: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    let before: String = chars[..cursor].iter().collect();
    let after: String = chars[cursor..].iter().collect();
    format!("{before}│{after}")
}
