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

    let popup_width = 36.min(area.width.saturating_sub(4));
    let popup_height = 6;

    let popup_area = centered_rect(popup_width, popup_height, area);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Clear Diagram ")
        .title_style(theme.accent)
        .borders(Borders::ALL)
        .border_style(theme.border);

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let message_area = Rect {
        x: inner.x,
        y: inner.y + 1,
        width: inner.width,
        height: 1,
    };
    frame.render_widget(
        Paragraph::new("Clear the entire diagram?")
            .style(theme.text)
            .alignment(Alignment::Center),
        message_area,
    );

    let hints = Line::from(vec![Span::styled("y/Enter: yes  n/Esc: No", theme.muted)]);

    let button_area = Rect {
        x: inner.x,
        y: inner.y + 3,
        width: inner.width,
        height: 1,
    };
    frame.render_widget(
        Paragraph::new(hints).alignment(Alignment::Center),
        button_area,
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
