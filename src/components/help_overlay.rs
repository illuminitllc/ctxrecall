use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::action::Action;
use crate::config::theme::Theme;
use crate::widgets::modal;

use super::Component;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpContext {
    IssueList,
    DetailPanel,
}

pub struct HelpOverlay {
    visible: bool,
    context: HelpContext,
}

impl HelpOverlay {
    pub fn new() -> Self {
        Self {
            visible: false,
            context: HelpContext::IssueList,
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn show(&mut self, context: HelpContext) {
        self.context = context;
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    fn bindings(&self) -> Vec<(&str, &str)> {
        match self.context {
            HelpContext::IssueList => vec![
                ("j/k", "Move up/down"),
                ("Enter", "View issue detail"),
                ("e", "Edit issue"),
                ("s", "Cycle status"),
                ("f", "Filter by status"),
                ("t", "Filter by team"),
                ("p", "Filter by project"),
                ("n", "New issue"),
                ("a", "Switch account"),
                ("c", "Launch Claude session"),
                ("T", "View transcripts"),
                ("d", "View documents"),
                ("r", "Refresh issues"),
                ("/", "Search"),
                ("h", "Show this help"),
                ("Ctrl-d", "Set status: Done"),
                ("Ctrl-b", "Set status: Backlog"),
                ("Ctrl-t", "Set status: Todo"),
                ("Ctrl-i", "Set status: In Progress"),
                ("Ctrl-r", "Cycle pane size"),
                ("Ctrl-s", "Settings"),
                ("Ctrl-p", "Command palette"),
                ("q", "Quit"),
            ],
            HelpContext::DetailPanel => vec![
                ("j/k", "Scroll up/down"),
                ("e", "Edit issue"),
                ("s", "Cycle status"),
                ("t", "Filter by team"),
                ("p", "Filter by project"),
                ("n", "New issue"),
                ("a", "Switch account"),
                ("c", "Launch Claude session"),
                ("T", "View transcripts"),
                ("d", "View documents"),
                ("r", "Refresh"),
                ("/", "Search"),
                ("h", "Show this help"),
                ("Ctrl-d", "Set status: Done"),
                ("Ctrl-b", "Set status: Backlog"),
                ("Ctrl-t", "Set status: Todo"),
                ("Ctrl-i", "Set status: In Progress"),
                ("Esc/q", "Back to list"),
            ],
        }
    }
}

impl Component for HelpOverlay {
    fn handle_key_event(&mut self, _key: KeyEvent) -> Option<Action> {
        // Any key dismisses the help overlay
        self.hide();
        Some(Action::HideHelp)
    }

    fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.visible {
            return;
        }

        let s = theme.styles();

        let title = match self.context {
            HelpContext::IssueList => "Help: Issue List",
            HelpContext::DetailPanel => "Help: Detail Panel",
        };

        // Scale modal to fit: use most of the available area
        let pct_x = if area.width < 60 { 95 } else { 70 };
        let pct_y = if area.height < 30 { 90 } else { 70 };
        let inner = modal::render_modal_themed(frame, area, title, pct_x, pct_y, Some(&s));

        let key_style = Style::default().fg(s.accent).add_modifier(Modifier::BOLD);

        let bindings = self.bindings();
        let mut lines: Vec<Line> = Vec::with_capacity(bindings.len() + 2);

        for (key, desc) in bindings {
            lines.push(Line::from(vec![
                Span::styled(format!(" {key:<7}"), key_style),
                Span::raw(desc),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            " Press any key to close",
            Style::default().fg(s.muted),
        )));

        frame.render_widget(Paragraph::new(lines), inner);
    }
}
