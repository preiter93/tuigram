#![allow(clippy::cast_possible_truncation)]

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use std::collections::HashMap;
use tui_world::{Keybindings, WidgetId};

use crate::theme::Theme;

const HELP_WIDTH: u16 = 56;

pub fn render_help(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    keybindings: &Keybindings,
    active: &[WidgetId],
) {
    let display = keybindings.display_for(active);

    // Group bindings by name, preserving order of first occurrence
    let mut grouped: HashMap<&'static str, Vec<String>> = HashMap::new();
    let mut order: Vec<&'static str> = Vec::new();
    for info in &display {
        let key_str = shorten_key(&info.key.display());
        if !grouped.contains_key(info.name) {
            order.push(info.name);
        }
        grouped.entry(info.name).or_default().push(key_str);
    }

    let lines: Vec<Line> = order
        .iter()
        .map(|name| {
            let keys = grouped.get(name).unwrap().join("/");
            Line::from(vec![
                Span::styled(format!("{:>14}", keys), theme.key),
                Span::raw("  "),
                Span::styled(*name, theme.text),
            ])
        })
        .collect();

    let popup_height = (lines.len() as u16 + 2).min(area.height.saturating_sub(2));

    let popup_area = centered_rect(HELP_WIDTH, popup_height, area);

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

fn shorten_key(key: &str) -> String {
    key.replace("Shift+BackTab", "S-Tab")
        .replace("Backspace", "Bksp")
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
