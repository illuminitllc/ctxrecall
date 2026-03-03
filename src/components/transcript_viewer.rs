use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, ListState, Paragraph, Tabs, Wrap};

use crate::action::Action;
use crate::config::theme::Theme;
use crate::widgets::modal;

use super::Component;

enum Tab {
    Transcripts,
    Summaries,
}

pub struct TranscriptViewer {
    visible: bool,
    issue_id: Option<String>,
    transcript_dir: std::path::PathBuf,
    files: Vec<(String, std::path::PathBuf)>, // (display name, path)
    selected: ListState,
    content: String,
    scroll: u16,
    tab: Tab,
}

impl TranscriptViewer {
    pub fn new(transcript_dir: std::path::PathBuf) -> Self {
        Self {
            visible: false,
            issue_id: None,
            transcript_dir,
            files: Vec::new(),
            selected: ListState::default(),
            content: String::new(),
            scroll: 0,
            tab: Tab::Transcripts,
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn show(&mut self, issue_id: &str) {
        self.issue_id = Some(issue_id.to_string());
        self.scroll = 0;
        self.content.clear();
        self.visible = true;
        self.load_files();
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    fn load_files(&mut self) {
        self.files.clear();
        let Some(issue_id) = &self.issue_id else {
            return;
        };

        let dir = self.transcript_dir.join(issue_id);
        let extension = match self.tab {
            Tab::Transcripts => "txt",
            Tab::Summaries => "summary.md",
        };

        if let Ok(entries) = std::fs::read_dir(&dir) {
            let mut files: Vec<_> = entries
                .flatten()
                .filter(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    match self.tab {
                        Tab::Transcripts => name.ends_with(".txt") && !name.contains(".summary"),
                        Tab::Summaries => name.ends_with(".summary.md"),
                    }
                })
                .map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    (name, e.path())
                })
                .collect();

            files.sort_by(|a, b| b.0.cmp(&a.0)); // newest first
            self.files = files;
        }

        if !self.files.is_empty() {
            self.selected.select(Some(0));
            self.load_content(0);
        } else {
            self.selected.select(None);
            self.content = format!("No {extension} files found for this issue.");
        }
    }

    fn load_content(&mut self, index: usize) {
        if let Some((_, path)) = self.files.get(index) {
            self.content = std::fs::read_to_string(path).unwrap_or_else(|e| format!("Error: {e}"));
            self.scroll = 0;
        }
    }
}

impl Component for TranscriptViewer {
    fn handle_key_event(&mut self, key: KeyEvent) -> Option<Action> {
        if !self.visible {
            return None;
        }

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.hide();
                None
            }
            KeyCode::Tab => {
                self.tab = match self.tab {
                    Tab::Transcripts => Tab::Summaries,
                    Tab::Summaries => Tab::Transcripts,
                };
                self.load_files();
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
            KeyCode::Char('n') => {
                // Next file
                let i = self.selected.selected().unwrap_or(0);
                if i + 1 < self.files.len() {
                    self.selected.select(Some(i + 1));
                    self.load_content(i + 1);
                }
                None
            }
            KeyCode::Char('p') => {
                // Prev file
                let i = self.selected.selected().unwrap_or(0);
                if i > 0 {
                    self.selected.select(Some(i - 1));
                    self.load_content(i - 1);
                }
                None
            }
            _ => None,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.visible {
            return;
        }

        let s = theme.styles();
        let inner = modal::render_modal_themed(frame, area, "Transcripts", 85, 80, Some(&s));

        let chunks = Layout::vertical([
            Constraint::Length(1), // Tab bar
            Constraint::Length(1), // File info
            Constraint::Min(1),   // Content
            Constraint::Length(1), // Help
        ])
        .split(inner);

        // Tab bar
        let tab_titles = vec!["Transcripts", "Summaries"];
        let selected_tab = match self.tab {
            Tab::Transcripts => 0,
            Tab::Summaries => 1,
        };
        let tabs = Tabs::new(tab_titles)
            .select(selected_tab)
            .style(Style::default().fg(s.muted))
            .highlight_style(Style::default().fg(s.warning).add_modifier(Modifier::BOLD));
        frame.render_widget(tabs, chunks[0]);

        // File info
        let file_info = if let Some(i) = self.selected.selected() {
            let (name, _) = &self.files[i];
            format!(" [{}/{}] {name}", i + 1, self.files.len())
        } else {
            " No files".to_string()
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                file_info,
                Style::default().fg(s.accent),
            ))),
            chunks[1],
        );

        // Content
        let content = Paragraph::new(Text::raw(&self.content))
            .wrap(Wrap { trim: false })
            .scroll((self.scroll, 0))
            .block(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(s.muted)),
            );
        frame.render_widget(content, chunks[2]);

        // Help
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                " Tab: switch | n/p: files | j/k: scroll | Esc: close",
                Style::default().fg(s.muted),
            ))),
            chunks[3],
        );
    }
}
