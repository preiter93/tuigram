use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::theme::Theme;

pub fn render_input_popup(
    frame: &mut Frame,
    title: &str,
    input: &str,
    prompt: &str,
    theme: &Theme,
) {
    let area = frame.area();

    let popup_width = 50.min(area.width.saturating_sub(4));
    let popup_height = 5;

    let popup_area = centered_rect(popup_width, popup_height, area);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_style(theme.border);

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let prompt_area = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: 1,
    };
    frame.render_widget(Paragraph::new(prompt).style(theme.muted), prompt_area);

    let input_area = Rect {
        x: inner.x,
        y: inner.y + 1,
        width: inner.width,
        height: 1,
    };

    let input_line = Line::from(vec![
        Span::styled(input, theme.text),
        Span::styled("█", theme.accent),
    ]);
    frame.render_widget(Paragraph::new(input_line), input_area);

    let hint_area = Rect {
        x: inner.x,
        y: inner.y + 2,
        width: inner.width,
        height: 1,
    };
    frame.render_widget(
        Paragraph::new("Enter: confirm | Esc: cancel")
            .style(theme.muted)
            .alignment(Alignment::Right),
        hint_area,
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
