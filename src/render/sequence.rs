#![allow(clippy::cast_possible_truncation, clippy::too_many_lines)]

use ratatui::{
    Frame,
    layout::{Alignment, Margin, Rect},
    text::Line,
    widgets::{Block, Borders, Paragraph},
};

use crate::{
    core::{Event, SequenceDiagram},
    theme::Theme,
    ui::Selection,
};

const HEADER_HEIGHT: u16 = 3;
const MESSAGE_SPACING: u16 = 3;

pub fn render_sequence(
    f: &mut Frame,
    outer_area: Rect,
    diagram: &SequenceDiagram,
    selection: Selection,
    theme: &Theme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border);
    f.render_widget(block, outer_area);

    let area = outer_area.inner(Margin::new(0, 1));
    let count = diagram.participants.len().max(1) as u16;
    let spacing = area.width / (count + 1);

    // Render participants
    let positions: Vec<u16> = (0..diagram.participants.len())
        .map(|i| spacing * (i as u16 + 1))
        .collect();

    for (i, name) in diagram.participants.iter().enumerate() {
        let x = positions[i];
        let is_selected = selection == Selection::Participant(i);
        let style = if is_selected {
            theme.selected
        } else {
            theme.text
        };

        let width = (name.len() as u16).saturating_add(4);
        let box_x = x
            .saturating_sub(width / 2)
            .min(area.width.saturating_sub(width));
        let box_area = Rect {
            x: box_x,
            y: area.y,
            width,
            height: 3,
        };

        f.render_widget(
            Block::default().borders(Borders::ALL).border_style(style),
            box_area,
        );
        f.render_widget(
            Paragraph::new(name.as_str())
                .alignment(Alignment::Center)
                .style(style),
            Rect {
                x: box_x + 1,
                y: area.y + 1,
                width: width - 2,
                height: 1,
            },
        );
    }

    // Render lifelines
    let lifeline_start = area.y + HEADER_HEIGHT;
    for &x in &positions {
        let xi = x.min(area.width - 1);
        for y in lifeline_start..area.y + area.height {
            f.render_widget(
                Paragraph::new("│").style(theme.text),
                Rect {
                    x: xi,
                    y,
                    width: 1,
                    height: 1,
                },
            );
        }
    }

    // Render messages
    for (i, event) in diagram.events.iter().enumerate() {
        let Event::Message { from, to, text } = event;
        let from_x = positions[*from];
        let to_x = positions[*to];
        let y = lifeline_start + HEADER_HEIGHT + (i as u16 * MESSAGE_SPACING);

        let is_selected = selection == Selection::Event(i);
        let style = if is_selected {
            theme.selected
        } else {
            theme.text
        };

        let start = from_x.min(to_x);
        let end = from_x.max(to_x);
        let len = end - start;

        let mut arrow = "─".repeat(len as usize);
        if from_x < to_x {
            arrow.push('>');
        } else {
            arrow.insert(0, '<');
        }

        f.render_widget(
            Paragraph::new(Line::from(arrow)).style(style),
            Rect {
                x: start,
                y,
                width: len + 1,
                height: 1,
            },
        );

        let text_width = text.len() as u16;
        let text_x = start + len.div_ceil(2);
        let text_start = text_x.saturating_sub(text_width / 2);

        f.render_widget(
            Paragraph::new(text.as_str())
                .alignment(Alignment::Center)
                .style(style),
            Rect {
                x: text_start,
                y: y.saturating_sub(1),
                width: text_width,
                height: 1,
            },
        );
    }
}
