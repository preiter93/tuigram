use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use tui_world::World;

use crate::theme::Theme;

pub fn render_confirm_dialog(frame: &mut Frame, world: &World) {
    let theme = world.get::<Theme>();

    let area = frame.area();

    let popup_width = 32.min(area.width.saturating_sub(4));
    let popup_height = 5;

    let popup_area = centered_rect(popup_width, popup_height, area);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Clear Diagram ")
        .title_alignment(Alignment::Center)
        .title_style(theme.accent)
        .borders(Borders::ALL)
        .border_style(theme.border);

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    frame.render_widget(
        Paragraph::new("Clear the entire diagram?")
            .style(theme.text)
            .alignment(Alignment::Center),
        Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: 1,
        },
    );

    let hints = Line::from(vec![
        Span::styled("[y]", theme.selected),
        Span::styled("es  ", theme.muted),
        Span::styled("[n]", theme.selected),
        Span::styled("o", theme.muted),
    ]);

    frame.render_widget(
        Paragraph::new(hints).alignment(Alignment::Center),
        Rect {
            x: inner.x,
            y: inner.y + 2,
            width: inner.width,
            height: 1,
        },
    );
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
