use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::action::DashboardStats;
use crate::config::theme::Theme;

use super::Component;

pub struct Dashboard {
    stats: DashboardStats,
}

impl Dashboard {
    pub fn new() -> Self {
        Self {
            stats: DashboardStats::default(),
        }
    }

    pub fn set_stats(&mut self, stats: DashboardStats) {
        self.stats = stats;
    }
}

impl Component for Dashboard {
    fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let s = theme.styles();
        let block = Block::default()
            .title(" Dashboard ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(s.border));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let cols = Layout::horizontal([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(inner);

        let bold = Style::default().add_modifier(Modifier::BOLD);
        let dim = Style::default().fg(s.muted);
        let accent = Style::default().fg(s.accent);
        let success = Style::default().fg(s.success);

        // Left column: Issue stats
        let issue_lines = vec![
            Line::from(Span::styled("Issue Stats", bold)),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Open:       ", dim),
                Span::styled(self.stats.open_count.to_string(), accent),
            ]),
            Line::from(vec![
                Span::styled("  Closed (7d): ", dim),
                Span::styled(self.stats.closed_7d_count.to_string(), success),
            ]),
        ];
        frame.render_widget(Paragraph::new(issue_lines), cols[0]);

        // Right column: Claude session stats
        let active_str = self
            .stats
            .active_session
            .as_deref()
            .unwrap_or("None");
        let last_issue_str = self
            .stats
            .last_session_issue
            .as_deref()
            .unwrap_or("—");
        let last_time_str = self
            .stats
            .last_session_time
            .as_deref()
            .unwrap_or("—");

        let session_lines = vec![
            Line::from(Span::styled("Claude Sessions", bold)),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Total:    ", dim),
                Span::styled(self.stats.total_sessions.to_string(), accent),
            ]),
            Line::from(vec![
                Span::styled("  Active:   ", dim),
                Span::styled(
                    active_str.to_string(),
                    if self.stats.active_session.is_some() {
                        success
                    } else {
                        dim
                    },
                ),
            ]),
            Line::from(vec![
                Span::styled("  Last:     ", dim),
                Span::raw(format!("{last_issue_str} ({last_time_str})")),
            ]),
        ];
        frame.render_widget(Paragraph::new(session_lines), cols[1]);
    }
}
