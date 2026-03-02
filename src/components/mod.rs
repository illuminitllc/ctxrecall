pub mod account_picker;
pub mod command_palette;
pub mod dashboard;
pub mod document_viewer;
pub mod filter_picker;
pub mod help_overlay;
pub mod issue_create;
pub mod issue_detail;
pub mod issue_edit;
pub mod issue_list;
pub mod search;
pub mod settings;
pub mod status_bar;
pub mod transcript_viewer;

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::action::Action;

pub trait Component {
    fn handle_key_event(&mut self, key: KeyEvent) -> Option<Action> {
        let _ = key;
        None
    }

    fn update(&mut self, action: &Action) -> Option<Action> {
        let _ = action;
        None
    }

    fn render(&self, frame: &mut Frame, area: Rect);
}
