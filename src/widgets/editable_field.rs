use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::config::theme::ThemeStyles;

pub struct EditableField {
    label: String,
    value: String,
    cursor: usize,
    editing: bool,
    multiline: bool,
}

impl EditableField {
    pub fn new(label: &str, value: &str, multiline: bool) -> Self {
        Self {
            label: label.to_string(),
            value: value.to_string(),
            cursor: value.len(),
            editing: false,
            multiline,
        }
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn set_value(&mut self, value: &str) {
        self.value = value.to_string();
        self.cursor = self.value.len();
    }

    pub fn is_editing(&self) -> bool {
        self.editing
    }

    pub fn cursor_pos(&self) -> usize {
        self.cursor
    }

    pub fn start_editing(&mut self) {
        self.editing = true;
        self.cursor = self.value.len();
    }

    pub fn stop_editing(&mut self) {
        self.editing = false;
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> EditFieldAction {
        if !self.editing {
            return EditFieldAction::None;
        }

        match key.code {
            KeyCode::Esc => {
                self.editing = false;
                EditFieldAction::Cancel
            }
            KeyCode::Enter => {
                if self.multiline {
                    self.insert_char('\n');
                    EditFieldAction::None
                } else {
                    self.editing = false;
                    EditFieldAction::Submit
                }
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    let byte_idx = self.byte_index(self.cursor - 1);
                    let next_byte_idx = self.byte_index(self.cursor);
                    self.value.replace_range(byte_idx..next_byte_idx, "");
                    self.cursor -= 1;
                }
                EditFieldAction::None
            }
            KeyCode::Delete => {
                let char_count = self.value.chars().count();
                if self.cursor < char_count {
                    let byte_idx = self.byte_index(self.cursor);
                    let next_byte_idx = self.byte_index(self.cursor + 1);
                    self.value.replace_range(byte_idx..next_byte_idx, "");
                }
                EditFieldAction::None
            }
            KeyCode::Left => {
                self.cursor = self.cursor.saturating_sub(1);
                EditFieldAction::None
            }
            KeyCode::Right => {
                let char_count = self.value.chars().count();
                if self.cursor < char_count {
                    self.cursor += 1;
                }
                EditFieldAction::None
            }
            KeyCode::Home => {
                self.cursor = 0;
                EditFieldAction::None
            }
            KeyCode::End => {
                self.cursor = self.value.chars().count();
                EditFieldAction::None
            }
            KeyCode::Char('v') if self.multiline && key.modifiers.contains(KeyModifiers::CONTROL) => {
                EditFieldAction::OpenExternal
            }
            KeyCode::Char(c) => {
                self.insert_char(c);
                EditFieldAction::None
            }
            _ => EditFieldAction::None,
        }
    }

    fn insert_char(&mut self, c: char) {
        let byte_idx = self.byte_index(self.cursor);
        self.value.insert(byte_idx, c);
        self.cursor += 1;
    }

    fn byte_index(&self, char_idx: usize) -> usize {
        self.value
            .char_indices()
            .nth(char_idx)
            .map(|(i, _)| i)
            .unwrap_or(self.value.len())
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        self.render_themed(frame, area, None);
    }

    pub fn render_themed(&self, frame: &mut Frame, area: Rect, styles: Option<&ThemeStyles>) {
        let accent = styles.map_or(Color::Cyan, |s| s.accent);
        let fg = styles.map_or(Color::White, |s| s.fg);
        let selection = styles.map_or(Color::DarkGray, |s| s.selection);

        let label_style = Style::default().fg(accent).add_modifier(Modifier::BOLD);
        let value_style = if self.editing {
            Style::default().fg(fg).bg(selection)
        } else {
            Style::default().fg(fg)
        };

        let display_val = if self.value.is_empty() && !self.editing {
            "(empty)".to_string()
        } else if self.editing {
            // Show cursor
            let chars: Vec<char> = self.value.chars().collect();
            let before: String = chars[..self.cursor].iter().collect();
            let cursor_char = chars.get(self.cursor).copied().unwrap_or(' ');
            let after: String = if self.cursor < chars.len() {
                chars[self.cursor + 1..].iter().collect()
            } else {
                String::new()
            };
            format!("{before}{cursor_char}{after}")
        } else {
            self.value.clone()
        };

        let line = Line::from(vec![
            Span::styled(format!("{}: ", self.label), label_style),
            Span::styled(display_val, value_style),
        ]);

        frame.render_widget(Paragraph::new(line), area);
    }
}

#[derive(Debug, PartialEq)]
pub enum EditFieldAction {
    None,
    Submit,
    Cancel,
    OpenExternal,
}
