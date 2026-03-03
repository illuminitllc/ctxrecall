use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Clear};

use crate::config::theme::ThemeStyles;

pub fn render_modal(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    percent_x: u16,
    percent_y: u16,
) -> Rect {
    render_modal_themed(frame, area, title, percent_x, percent_y, None)
}

pub fn render_modal_themed(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    percent_x: u16,
    percent_y: u16,
    styles: Option<&ThemeStyles>,
) -> Rect {
    let popup_area = centered_rect(percent_x, percent_y, area);
    frame.render_widget(Clear, popup_area);

    let border_color = styles.map_or(Color::Cyan, |s| s.accent);
    let block = Block::default()
        .title(format!(" {title} "))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);
    inner
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}
