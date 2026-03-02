use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::action::Action;

use super::Component;

pub struct StatusBar {
    message: String,
    is_error: bool,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            message: "ctxrecall v0.1.0 | q: quit | j/k: navigate | /: search | c: claude | C-p: commands".into(),
            is_error: false,
        }
    }
}

impl Component for StatusBar {
    fn update(&mut self, action: &Action) -> Option<Action> {
        match action {
            Action::StatusMessage(msg) => {
                self.message = msg.clone();
                self.is_error = false;
            }
            Action::Error(msg) => {
                self.message = format!("Error: {msg}");
                self.is_error = true;
            }
            _ => {}
        }
        None
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let color = if self.is_error {
            Color::Red
        } else {
            Color::DarkGray
        };

        let line = Line::from(vec![Span::styled(
            &self.message,
            Style::default().fg(color),
        )]);

        let paragraph = Paragraph::new(line)
            .style(Style::default().bg(Color::Black));

        frame.render_widget(paragraph, area);
    }
}
