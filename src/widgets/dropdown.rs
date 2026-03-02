use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState};

pub struct Dropdown {
    label: String,
    items: Vec<String>,
    selected: usize,
    open: bool,
    state: ListState,
}

impl Dropdown {
    pub fn new(label: &str, items: Vec<String>, selected: usize) -> Self {
        let mut state = ListState::default();
        state.select(Some(selected));
        Self {
            label: label.to_string(),
            items,
            selected,
            open: false,
            state,
        }
    }

    pub fn selected_value(&self) -> Option<&str> {
        self.items.get(self.selected).map(|s| s.as_str())
    }

    pub fn set_items(&mut self, items: Vec<String>, selected: usize) {
        self.items = items;
        self.selected = selected.min(self.items.len().saturating_sub(1));
        self.state.select(Some(self.selected));
    }

    pub fn set_selected(&mut self, idx: usize) {
        self.selected = idx.min(self.items.len().saturating_sub(1));
        self.state.select(Some(self.selected));
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
        if self.open {
            self.state.select(Some(self.selected));
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> DropdownAction {
        if !self.open {
            return DropdownAction::None;
        }

        match key.code {
            KeyCode::Esc => {
                self.open = false;
                DropdownAction::Cancel
            }
            KeyCode::Enter => {
                self.selected = self.state.selected().unwrap_or(0);
                self.open = false;
                DropdownAction::Selected(self.selected)
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let i = self.state.selected().unwrap_or(0);
                if i < self.items.len().saturating_sub(1) {
                    self.state.select(Some(i + 1));
                }
                DropdownAction::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let i = self.state.selected().unwrap_or(0);
                if i > 0 {
                    self.state.select(Some(i - 1));
                }
                DropdownAction::None
            }
            _ => DropdownAction::None,
        }
    }

    pub fn render_inline(&self, frame: &mut Frame, area: Rect) {
        let display = self
            .selected_value()
            .unwrap_or("(none)")
            .to_string();

        let style = if self.open {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let line = Line::from(vec![
            Span::styled(
                format!("{}: ", self.label),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("{display} [v]"), style),
        ]);

        frame.render_widget(ratatui::widgets::Paragraph::new(line), area);
    }

    pub fn render_popup(&self, frame: &mut Frame, area: Rect) {
        if !self.open {
            return;
        }

        let height = (self.items.len() as u16 + 2).min(area.height);
        let width = self
            .items
            .iter()
            .map(|s| s.len() as u16)
            .max()
            .unwrap_or(10)
            + 4;
        let width = width.min(area.width);

        let popup = Rect {
            x: area.x,
            y: area.y,
            width,
            height,
        };

        frame.render_widget(Clear, popup);

        let items: Vec<ListItem> = self
            .items
            .iter()
            .map(|s| ListItem::new(s.as_str()))
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title(format!(" {} ", self.label))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(list, popup, &mut self.state.clone());
    }
}

#[derive(Debug, PartialEq)]
pub enum DropdownAction {
    None,
    Selected(usize),
    Cancel,
}
