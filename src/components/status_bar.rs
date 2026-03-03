use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::action::Action;
use crate::config::theme::Theme;

use super::Component;

pub struct StatusBar {
    message: Option<String>,
    is_error: bool,
    context_hints: String,
}

impl StatusBar {
    pub fn new() -> Self {
        let mut bar = Self {
            message: None,
            is_error: false,
            context_hints: String::new(),
        };
        bar.set_context("list");
        bar
    }

    pub fn clear_transient(&mut self) {
        self.message = None;
    }

    pub fn set_context(&mut self, context: &str) {
        self.context_hints = match context {
            "list" => "j/k:nav | e:edit | s:status | C-d/b/t/i:set status | f:filter | n:new | /:search | h:help".into(),
            "detail" => "j/k:scroll | e:edit | s:status | C-d/b/t/i:set status | c:claude | T:transcripts | h:help".into(),
            _ => "j/k:nav | e:edit | s:status | C-d/b/t/i:set status | f:filter | n:new | /:search | h:help".into(),
        };
    }
}

impl Component for StatusBar {
    fn update(&mut self, action: &Action) -> Option<Action> {
        match action {
            Action::StatusMessage(msg) => {
                self.message = Some(msg.clone());
                self.is_error = false;
            }
            Action::Error(msg) => {
                self.message = Some(format!("Error: {msg}"));
                self.is_error = true;
            }
            _ => {}
        }
        None
    }

    fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let s = theme.styles();
        let (text, color) = if let Some(ref msg) = self.message {
            (msg.as_str(), if self.is_error { s.error } else { s.muted })
        } else {
            (self.context_hints.as_str(), s.muted)
        };

        let line = Line::from(vec![Span::styled(
            text,
            Style::default().fg(color),
        )]);

        let paragraph = Paragraph::new(line)
            .style(Style::default().bg(s.bg));

        frame.render_widget(paragraph, area);
    }
}
