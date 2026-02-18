#![allow(clippy::cast_possible_truncation, clippy::too_many_lines)]

use ratatui::{
    Frame,
    layout::{Alignment, Margin, Rect},
    text::Line,
    widgets::{Block, Borders, Paragraph},
};
use tui_world::World;

use crate::{
    core::{Event, SequenceDiagram},
    theme::Theme,
    ui::{EditorState, Selection},
};

const HEADER_HEIGHT: u16 = 3;
const MESSAGE_SPACING: u16 = 3;
const FIRST_MESSAGE_OFFSET: u16 = 2;

pub fn render_sequence(f: &mut Frame, outer_area: Rect, world: &World) {
    let diagram = world.get::<SequenceDiagram>();
    let selection = world.get::<EditorState>().selection;
    let theme = world.get::<Theme>();

    f.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border),
        outer_area,
    );

    let area = outer_area.inner(Margin::new(0, 1));

    let participants = render_participants(f, area, diagram, selection, theme);
    let lifeline_start = render_lifelines(f, area, theme, &participants);
    render_events(f, diagram, selection, theme, &participants, lifeline_start);
}

pub fn render_participants(
    f: &mut Frame,
    area: Rect,
    diagram: &SequenceDiagram,
    selection: Selection,
    theme: &Theme,
) -> Vec<u16> {
    let count = diagram.participants.len().max(1) as u16;
    let spacing = area.width / (count + 1);

    let positions: Vec<u16> = (0..diagram.participants.len())
        .map(|i| spacing * (i as u16 + 1))
        .collect();

    for (i, name) in diagram.participants.iter().enumerate() {
        let style = if selection == Selection::Participant(i) {
            theme.selected
        } else {
            theme.text
        };

        let width = name.len() as u16 + 4;
        let x = positions[i]
            .saturating_sub(width / 2)
            .min(area.width.saturating_sub(width))
            + 1;

        f.render_widget(
            Paragraph::new(name.as_str())
                .alignment(Alignment::Center)
                .style(style)
                .block(Block::default().borders(Borders::ALL).border_style(style)),
            Rect {
                x,
                y: area.y,
                width,
                height: 3,
            },
        );
    }

    positions
}

fn render_lifelines(f: &mut Frame, area: Rect, theme: &Theme, participants: &[u16]) -> u16 {
    let lifeline_start = area.y + HEADER_HEIGHT;
    for &x in participants {
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
    lifeline_start
}

pub fn render_events(
    f: &mut Frame,
    diagram: &SequenceDiagram,
    selection: Selection,
    theme: &Theme,
    participants: &[u16],
    lifeline_start: u16,
) {
    for (i, event) in diagram.events.iter().enumerate() {
        let Event::Message { from, to, text } = event;
        let from_x = participants[*from];
        let to_x = participants[*to];
        let y = lifeline_start + FIRST_MESSAGE_OFFSET + (i as u16 * MESSAGE_SPACING);

        let style = if selection == Selection::Event(i) {
            theme.selected
        } else {
            theme.text
        };

        if from == to {
            let loop_width: u16 = 4;

            let mut area = Rect {
                x: from_x,
                y: y.saturating_sub(1),
                width: loop_width,
                height: 1,
            };

            f.render_widget(Paragraph::new("───┐").style(style), area);

            area.y = y;
            f.render_widget(Paragraph::new("   │").style(style), area);

            area.y = y + 1;
            f.render_widget(Paragraph::new("◀──┘").style(style), area);

            area.x = from_x + loop_width;
            area.y = y;
            area.width = text.len() as u16;
            f.render_widget(Paragraph::new(text.as_str()).style(style), area);
        } else {
            let start = from_x.min(to_x);
            let end = from_x.max(to_x);
            let len = end - start;

            let mut arrow = "─".repeat(len as usize);
            if from_x < to_x {
                arrow.push('▶');
            } else {
                arrow.insert(0, '◀');
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
}
