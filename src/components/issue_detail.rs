use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::action::Action;
use crate::config::theme::{Theme, ThemeStyles};
use crate::tracker::types::Issue;

use super::Component;

pub struct IssueDetail {
    issue: Option<Issue>,
    scroll: u16,
    branch_info: Option<String>,
}

impl IssueDetail {
    pub fn new() -> Self {
        Self {
            issue: None,
            scroll: 0,
            branch_info: None,
        }
    }

    pub fn set_issue(&mut self, issue: Issue) {
        self.issue = Some(issue);
        self.scroll = 0;
        self.branch_info = None;
    }

    pub fn clear(&mut self) {
        self.issue = None;
        self.scroll = 0;
        self.branch_info = None;
    }

    pub fn set_branch_info(&mut self, info: Option<String>) {
        self.branch_info = info;
    }

    pub fn current_issue(&self) -> Option<&Issue> {
        self.issue.as_ref()
    }

    fn render_content_themed(&self, width: u16, s: &ThemeStyles) -> Text<'_> {
        let issue = match &self.issue {
            Some(i) => i,
            None => return Text::raw("No issue selected"),
        };

        let bold = Style::default().add_modifier(Modifier::BOLD);
        let dim = Style::default().fg(s.muted);
        let accent = Style::default().fg(s.accent);

        let mut lines = if width < 60 {
            // Compact layout for narrow panes
            let assignee = issue.assignee.as_deref().unwrap_or("—");
            let team = issue.team.as_deref().unwrap_or("—");

            let mut l = vec![
                Line::from(vec![
                    Span::styled(&issue.identifier, accent),
                    Span::raw("  "),
                    Span::raw(&issue.title),
                ]),
                Line::from(vec![
                    Span::styled(&issue.status, Style::default().fg(s.warning)),
                    Span::raw(" · "),
                    Span::raw(priority_label(issue.priority)),
                    Span::raw(" · "),
                    Span::styled(assignee, Style::default().fg(s.accent)),
                    Span::raw(" · "),
                    Span::raw(team),
                ]),
            ];

            if let Some(project) = &issue.project {
                l.push(Line::from(vec![
                    Span::styled("Project: ", dim),
                    Span::raw(project.as_str()),
                ]));
            }

            if !issue.labels.is_empty() {
                l.push(Line::from(vec![
                    Span::styled("Labels: ", dim),
                    Span::raw(issue.labels.join(", ")),
                ]));
            }

            l.push(Line::from(vec![
                Span::styled(
                    issue.updated_at.format("%Y-%m-%d %H:%M").to_string(),
                    dim,
                ),
                Span::styled(" · ", dim),
                Span::styled(&issue.url, Style::default().fg(s.border)),
            ]));
            l
        } else {
            // Spacious layout for wider panes
            let mut l = vec![
                Line::from(vec![
                    Span::styled("Identifier: ", bold),
                    Span::styled(&issue.identifier, accent),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Title: ", bold),
                    Span::raw(&issue.title),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Status: ", bold),
                    Span::raw(&issue.status),
                ]),
                Line::from(vec![
                    Span::styled("Priority: ", bold),
                    Span::raw(priority_label(issue.priority)),
                ]),
                Line::from(vec![
                    Span::styled("Assignee: ", bold),
                    Span::raw(issue.assignee.as_deref().unwrap_or("Unassigned")),
                ]),
                Line::from(vec![
                    Span::styled("Team: ", bold),
                    Span::raw(issue.team.as_deref().unwrap_or("None")),
                ]),
                Line::from(vec![
                    Span::styled("Project: ", bold),
                    Span::raw(issue.project.as_deref().unwrap_or("None")),
                ]),
            ];

            if !issue.labels.is_empty() {
                l.push(Line::from(vec![
                    Span::styled("Labels: ", bold),
                    Span::raw(issue.labels.join(", ")),
                ]));
            }

            l.push(Line::from(""));
            l.push(Line::from(vec![
                Span::styled("URL: ", bold),
                Span::styled(&issue.url, Style::default().fg(s.border)),
            ]));

            l.push(Line::from(""));
            l.push(Line::from(vec![
                Span::styled("Updated: ", dim),
                Span::styled(
                    issue.updated_at.format("%Y-%m-%d %H:%M").to_string(),
                    dim,
                ),
                Span::styled("  Created: ", dim),
                Span::styled(
                    issue.created_at.format("%Y-%m-%d %H:%M").to_string(),
                    dim,
                ),
            ]));
            l
        };

        // Description
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "─── Description ───",
            Style::default().fg(s.border),
        )));
        lines.push(Line::from(""));

        if let Some(desc) = &issue.description {
            for line in desc.lines() {
                lines.push(Line::from(line.to_string()));
            }
        } else {
            lines.push(Line::from(Span::styled("No description", dim)));
        }

        Text::from(lines)
    }
}

fn priority_label(p: i32) -> &'static str {
    match p {
        0 => "No priority",
        1 => "Urgent",
        2 => "High",
        3 => "Medium",
        4 => "Low",
        _ => "Unknown",
    }
}

impl Component for IssueDetail {
    fn handle_key_event(&mut self, key: KeyEvent) -> Option<Action> {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return match key.code {
                KeyCode::Char('d') => self.issue.as_ref().map(|i| Action::SetStatus(i.id.clone(), "Done".into())),
                KeyCode::Char('b') => self.issue.as_ref().map(|i| Action::SetStatus(i.id.clone(), "Backlog".into())),
                KeyCode::Char('t') => self.issue.as_ref().map(|i| Action::SetStatus(i.id.clone(), "Todo".into())),
                KeyCode::Char('i') => self.issue.as_ref().map(|i| Action::SetStatus(i.id.clone(), "In Progress".into())),
                _ => None,
            };
        }

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => Some(Action::Back),
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll = self.scroll.saturating_add(1);
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll = self.scroll.saturating_sub(1);
                None
            }
            KeyCode::Char('c') => {
                self.issue.as_ref().map(|i| Action::LaunchClaude(i.id.clone()))
            }
            KeyCode::Char('e') => {
                self.issue.as_ref().map(|i| Action::EditIssue(i.clone()))
            }
            KeyCode::Char('T') => {
                self.issue.as_ref().map(|i| Action::ViewTranscripts(i.id.clone()))
            }
            KeyCode::Char('d') => {
                self.issue.as_ref().map(|i| Action::ViewDocuments(i.id.clone()))
            }
            KeyCode::Char('s') => {
                self.issue.as_ref().map(|i| Action::CycleStatus(i.id.clone()))
            }
            KeyCode::Char('b') => {
                self.issue.as_ref().map(|i| Action::OpenBranchPicker(i.id.clone()))
            }
            KeyCode::Char('h') => Some(Action::ShowHelp),
            KeyCode::Char('/') => Some(Action::OpenSearch),
            KeyCode::Char('r') => Some(Action::Refresh),
            _ => None,
        }
    }

    fn update(&mut self, action: &Action) -> Option<Action> {
        if let Action::IssueSaved(updated) = action {
            if self.issue.as_ref().map(|i| &i.id) == Some(&updated.id) {
                self.issue = Some(updated.clone());
            }
        }
        None
    }

    fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let s = theme.styles();
        let title = self
            .issue
            .as_ref()
            .map(|i| format!(" {} - {} ", i.identifier, i.title))
            .unwrap_or_else(|| " Issue Detail ".to_string());

        let mut block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(s.accent));

        if let Some(ref info) = self.branch_info {
            let branch_title = Line::from(vec![
                Span::styled(
                    format!("  {info} "),
                    Style::default().fg(s.success),
                ),
            ]);
            block = block.title(branch_title.alignment(Alignment::Right));
        }

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let content = self.render_content_themed(inner.width, &s);
        let paragraph = Paragraph::new(content)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll, 0));

        frame.render_widget(paragraph, inner);
    }
}
