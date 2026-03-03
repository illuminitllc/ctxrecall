use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState};

use crate::action::Action;
use crate::config::theme::Theme;
use crate::widgets::modal;

use super::Component;

#[derive(Debug, Clone)]
pub struct Account {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub is_active: bool,
}

pub struct AccountPicker {
    visible: bool,
    accounts: Vec<Account>,
    state: ListState,
}

impl AccountPicker {
    pub fn new() -> Self {
        Self {
            visible: false,
            accounts: Vec::new(),
            state: ListState::default(),
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn show(&mut self, accounts: Vec<Account>) {
        self.accounts = accounts;
        if !self.accounts.is_empty() {
            let active_idx = self
                .accounts
                .iter()
                .position(|a| a.is_active)
                .unwrap_or(0);
            self.state.select(Some(active_idx));
        }
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }
}

impl Component for AccountPicker {
    fn handle_key_event(&mut self, key: KeyEvent) -> Option<Action> {
        if !self.visible {
            return None;
        }

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.hide();
                None
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let i = self.state.selected().unwrap_or(0);
                if i < self.accounts.len().saturating_sub(1) {
                    self.state.select(Some(i + 1));
                }
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let i = self.state.selected().unwrap_or(0);
                if i > 0 {
                    self.state.select(Some(i - 1));
                }
                None
            }
            KeyCode::Enter => {
                let action = self.state.selected().and_then(|i| {
                    self.accounts.get(i).map(|account| {
                        Action::SwitchAccount(account.id.clone())
                    })
                });
                self.hide();
                action
            }
            _ => None,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.visible {
            return;
        }

        let s = theme.styles();
        let inner = modal::render_modal_themed(frame, area, "Switch Account", 40, 30, Some(&s));

        if self.accounts.is_empty() {
            return;
        }

        let items: Vec<ListItem> = self
            .accounts
            .iter()
            .map(|a| {
                let active = if a.is_active { " *" } else { "  " };
                ListItem::new(Line::from(vec![
                    Span::styled(active, Style::default().fg(s.success)),
                    Span::raw(format!(" {} ", a.name)),
                    Span::styled(
                        format!("({})", a.provider),
                        Style::default().fg(s.muted),
                    ),
                ]))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(s.selection)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(list, inner, &mut self.state.clone());
    }
}
