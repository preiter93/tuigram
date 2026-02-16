#![allow(clippy::cast_possible_truncation)]

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use tui_world::{Keybindings, WidgetId};

use crate::theme::Theme;

pub fn render_help(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    keybindings: &Keybindings,
    active: &[WidgetId],
) {
    let display = keybindings.display_for(active);

    let lines: Vec<Line> = display
        .iter()
        .map(|info| {
            Line::from(vec![
                Span::styled(format!("{:>14}", info.key.display()), theme.key),
                Span::raw("  "),
                Span::styled(info.name, theme.text),
            ])
        })
        .collect();

    let popup_width = 46u16;
    let popup_height = (lines.len() as u16 + 2).min(area.height.saturating_sub(2));

    let popup_area = centered_rect(popup_width, popup_height, area);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Help ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(theme.border);

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let [area] = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .areas(area);
    area
}
