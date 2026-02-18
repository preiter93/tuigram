#![allow(clippy::cast_possible_truncation, clippy::too_many_lines)]

use ratatui::{
    Frame,
    layout::{Alignment, Margin, Rect},
    text::Line,
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation},
};
use tui_world::World;

use crate::{
    core::{Event, SequenceDiagram},
    theme::Theme,
    ui::{
        EditorState, FIRST_MESSAGE_OFFSET, HEADER_HEIGHT, MESSAGE_SPACING, Selection,
        scroll::ScrollState,
    },
};

pub fn render_sequence(f: &mut Frame, outer_area: Rect, world: &mut World) {
    let selection = world.get::<EditorState>().selection;
    let area = outer_area.inner(Margin::new(0, 1));

    world.get_mut::<ScrollState>().set_viewport(area.height);

    if let Selection::Event(idx) = selection {
        world.get_mut::<ScrollState>().ensure_visible(idx);
    }

    let theme = world.get::<Theme>();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border);
    f.render_widget(block, outer_area);

    let participants = render_participants(f, area, world);
    let lifeline_start = area.y + HEADER_HEIGHT;
    render_lifelines(f, area, world, &participants, lifeline_start);
    render_events(f, world, &participants, lifeline_start);
    render_scrollbar(f, area, world);
}

fn render_scrollbar(f: &mut Frame, area: Rect, world: &World) {
    let diagram = world.get::<SequenceDiagram>();
    let scroll = world.get::<ScrollState>();
    let event_count = diagram.event_count();

    if !scroll.needs_scroll(event_count) {
        return;
    }

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
    let scrollbar_area = Rect {
        x: area.x,
        y: area.y + HEADER_HEIGHT,
        width: area.width,
        height: area.height.saturating_sub(HEADER_HEIGHT),
    };
    let mut scrollbar_state = scroll.scrollbar_state(event_count);
    f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
}

fn render_participants(f: &mut Frame, area: Rect, world: &World) -> Vec<u16> {
    let diagram = world.get::<SequenceDiagram>();
    let selection = world.get::<EditorState>().selection;
    let theme = world.get::<Theme>();

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

fn render_lifelines(
    f: &mut Frame,
    area: Rect,
    world: &World,
    participants: &[u16],
    lifeline_start: u16,
) {
    let diagram = world.get::<SequenceDiagram>();
    let scroll = world.get::<ScrollState>();
    let theme = world.get::<Theme>();
    let event_count = diagram.event_count();

    let visible_range = scroll.visible_range(event_count);
    let has_above = visible_range.start > 0;
    let has_below = visible_range.end < event_count;

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

        // Show scroll indicators on lifelines
        if has_above {
            f.render_widget(
                Paragraph::new("⋮").style(theme.muted),
                Rect {
                    x: xi,
                    y: lifeline_start,
                    width: 1,
                    height: 1,
                },
            );
        }

        if has_below {
            f.render_widget(
                Paragraph::new("⋮").style(theme.muted),
                Rect {
                    x: xi,
                    y: area.y + area.height - 1,
                    width: 1,
                    height: 1,
                },
            );
        }
    }
}

fn render_events(f: &mut Frame, world: &World, participants: &[u16], lifeline_start: u16) {
    let diagram = world.get::<SequenceDiagram>();
    let selection = world.get::<EditorState>().selection;
    let theme = world.get::<Theme>();
    let scroll = world.get::<ScrollState>();
    let visible_range = scroll.visible_range(diagram.event_count());
    let scroll_offset = scroll.offset;

    for i in visible_range {
        let Some(event) = diagram.events.get(i) else {
            continue;
        };
        let Event::Message { from, to, text } = event;
        let from_x = participants[*from];
        let to_x = participants[*to];
        let visible_index = i - scroll_offset;
        let y = lifeline_start + FIRST_MESSAGE_OFFSET + (visible_index as u16 * MESSAGE_SPACING);

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
