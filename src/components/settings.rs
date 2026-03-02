use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph, Tabs};

use crate::action::Action;
use crate::config::hotkeys::HotkeyBinding;
use crate::widgets::modal;

use super::Component;

enum SettingsTab {
    General,
    Hotkeys,
    Theme,
    Accounts,
    Llm,
}

pub struct Settings {
    visible: bool,
    tab: SettingsTab,
    hotkeys: Vec<HotkeyBinding>,
    hotkey_state: ListState,
    themes: Vec<String>,
    theme_state: ListState,
    llm_provider: String,
    llm_model: String,
    llm_api_key_display: String,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            visible: false,
            tab: SettingsTab::General,
            hotkeys: Vec::new(),
            hotkey_state: ListState::default(),
            themes: vec![
                "dark".into(),
                "light".into(),
                "solarized".into(),
                "gruvbox".into(),
            ],
            theme_state: {
                let mut s = ListState::default();
                s.select(Some(0));
                s
            },
            llm_provider: String::new(),
            llm_model: String::new(),
            llm_api_key_display: String::new(),
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn show(&mut self, hotkeys: Vec<HotkeyBinding>) {
        self.hotkeys = hotkeys;
        self.tab = SettingsTab::General;
        self.visible = true;
        if !self.hotkeys.is_empty() {
            self.hotkey_state.select(Some(0));
        }
    }

    pub fn set_llm_config(&mut self, provider: Option<String>, model: Option<String>, api_key: Option<String>) {
        self.llm_provider = match provider.as_deref() {
            Some("claude") => "Claude API".into(),
            Some("openai") => "OpenAI".into(),
            Some("ollama") => "Ollama".into(),
            Some(other) => other.to_string(),
            None => "Not configured".into(),
        };
        self.llm_model = model.unwrap_or_else(|| "(default)".into());
        self.llm_api_key_display = match api_key {
            Some(k) if k.len() > 6 => {
                let prefix = &k[..3];
                let suffix = &k[k.len() - 3..];
                format!("{prefix}...{suffix}")
            }
            Some(_) => "(set)".into(),
            None => "(not set)".into(),
        };
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    fn next_tab(&mut self) {
        self.tab = match self.tab {
            SettingsTab::General => SettingsTab::Hotkeys,
            SettingsTab::Hotkeys => SettingsTab::Theme,
            SettingsTab::Theme => SettingsTab::Accounts,
            SettingsTab::Accounts => SettingsTab::Llm,
            SettingsTab::Llm => SettingsTab::General,
        };
    }

    fn prev_tab(&mut self) {
        self.tab = match self.tab {
            SettingsTab::General => SettingsTab::Llm,
            SettingsTab::Hotkeys => SettingsTab::General,
            SettingsTab::Theme => SettingsTab::Hotkeys,
            SettingsTab::Accounts => SettingsTab::Theme,
            SettingsTab::Llm => SettingsTab::Accounts,
        };
    }

    fn tab_index(&self) -> usize {
        match self.tab {
            SettingsTab::General => 0,
            SettingsTab::Hotkeys => 1,
            SettingsTab::Theme => 2,
            SettingsTab::Accounts => 3,
            SettingsTab::Llm => 4,
        }
    }
}

impl Component for Settings {
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
                self.next_tab();
                None
            }
            KeyCode::BackTab => {
                self.prev_tab();
                None
            }
            KeyCode::Char('j') | KeyCode::Down => {
                match self.tab {
                    SettingsTab::Hotkeys => {
                        let i = self.hotkey_state.selected().unwrap_or(0);
                        if i < self.hotkeys.len().saturating_sub(1) {
                            self.hotkey_state.select(Some(i + 1));
                        }
                    }
                    SettingsTab::Theme => {
                        let i = self.theme_state.selected().unwrap_or(0);
                        if i < self.themes.len().saturating_sub(1) {
                            self.theme_state.select(Some(i + 1));
                        }
                    }
                    _ => {}
                }
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                match self.tab {
                    SettingsTab::Hotkeys => {
                        let i = self.hotkey_state.selected().unwrap_or(0);
                        if i > 0 {
                            self.hotkey_state.select(Some(i - 1));
                        }
                    }
                    SettingsTab::Theme => {
                        let i = self.theme_state.selected().unwrap_or(0);
                        if i > 0 {
                            self.theme_state.select(Some(i - 1));
                        }
                    }
                    _ => {}
                }
                None
            }
            _ => None,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let inner = modal::render_modal(frame, area, "Settings", 70, 70);

        let chunks = Layout::vertical([
            Constraint::Length(1), // Tabs
            Constraint::Length(1), // Spacer
            Constraint::Min(1),   // Content
            Constraint::Length(1), // Help
        ])
        .split(inner);

        // Tab bar
        let tabs = Tabs::new(vec!["General", "Hotkeys", "Theme", "Accounts", "LLM"])
            .select(self.tab_index())
            .style(Style::default().fg(Color::DarkGray))
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_widget(tabs, chunks[0]);

        // Content based on tab
        match self.tab {
            SettingsTab::General => {
                frame.render_widget(
                    Paragraph::new(vec![
                        Line::from(Span::styled(
                            "  General Settings",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        )),
                        Line::from(""),
                        Line::from("  Sync interval: 120s"),
                        Line::from("  Transcript capture: 3s"),
                        Line::from("  Summary interval: 10min"),
                    ]),
                    chunks[2],
                );
            }
            SettingsTab::Hotkeys => {
                let items: Vec<ListItem> = self
                    .hotkeys
                    .iter()
                    .map(|h| {
                        ListItem::new(Line::from(vec![
                            Span::styled(
                                format!("{:<20}", h.action),
                                Style::default().fg(Color::Cyan),
                            ),
                            Span::styled(
                                format!("{:<10}", h.key_binding),
                                Style::default().fg(Color::Yellow),
                            ),
                            Span::styled(
                                h.description.as_deref().unwrap_or(""),
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

                frame.render_stateful_widget(
                    list,
                    chunks[2],
                    &mut self.hotkey_state.clone(),
                );
            }
            SettingsTab::Theme => {
                let items: Vec<ListItem> = self
                    .themes
                    .iter()
                    .map(|t| ListItem::new(format!("  {t}")))
                    .collect();

                let list = List::new(items)
                    .highlight_style(
                        Style::default()
                            .bg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol("▶ ");

                frame.render_stateful_widget(
                    list,
                    chunks[2],
                    &mut self.theme_state.clone(),
                );
            }
            SettingsTab::Accounts => {
                frame.render_widget(
                    Paragraph::new(vec![
                        Line::from(Span::styled(
                            "  Accounts",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        )),
                        Line::from(""),
                        Line::from("  No accounts configured."),
                        Line::from("  Use --linear-api-key or LINEAR_API_KEY env var."),
                    ]),
                    chunks[2],
                );
            }
            SettingsTab::Llm => {
                frame.render_widget(
                    Paragraph::new(vec![
                        Line::from(Span::styled(
                            "  LLM Settings",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        )),
                        Line::from(""),
                        Line::from(vec![
                            Span::styled("  Provider: ", Style::default().fg(Color::DarkGray)),
                            Span::styled(&self.llm_provider, Style::default().fg(Color::White)),
                        ]),
                        Line::from(vec![
                            Span::styled("  Model:    ", Style::default().fg(Color::DarkGray)),
                            Span::styled(&self.llm_model, Style::default().fg(Color::White)),
                        ]),
                        Line::from(vec![
                            Span::styled("  API Key:  ", Style::default().fg(Color::DarkGray)),
                            Span::styled(&self.llm_api_key_display, Style::default().fg(Color::White)),
                        ]),
                        Line::from(""),
                        Line::from(Span::styled(
                            "  Use --set-llm-provider to configure",
                            Style::default().fg(Color::DarkGray),
                        )),
                    ]),
                    chunks[2],
                );
            }
        }

        // Help
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                " Tab/S-Tab: sections | j/k: navigate | Esc: close",
                Style::default().fg(Color::DarkGray),
            ))),
            chunks[3],
        );
    }
}
