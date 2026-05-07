use ratatui::{
    Frame,
    layout::{Alignment, Margin, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation},
};
use tui_world::World;

use crate::{
    core::{BoxColor, Event, NotePosition, SequenceDiagram},
    theme::Theme,
    ui::{EditorState, FIRST_MESSAGE_OFFSET, HEADER_HEIGHT, Selection, scroll::ScrollState},
};

pub fn render_sequence(f: &mut Frame, outer_area: Rect, world: &mut World) {
    let selection = world.get::<EditorState>().selection;
    let area = outer_area.inner(Margin::new(0, 1));

    world.get_mut::<ScrollState>().set_viewport(area.height);

    if let Selection::Event(idx) = selection {
        let diagram = world.get::<SequenceDiagram>().clone();
        world.get_mut::<ScrollState>().ensure_visible(idx, &diagram);
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
    // Render box labels last so they sit on top of lifelines and events
    render_box_labels(f, area, world, &participants);
}

fn render_scrollbar(f: &mut Frame, area: Rect, world: &World) {
    let diagram = world.get::<SequenceDiagram>();
    let scroll = world.get::<ScrollState>();

    if !scroll.needs_scroll(diagram) {
        return;
    }

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
    let scrollbar_area = Rect {
        x: area.x,
        y: area.y + HEADER_HEIGHT,
        width: area.width,
        height: area.height.saturating_sub(HEADER_HEIGHT),
    };
    let mut scrollbar_state = scroll.scrollbar_state(diagram);
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

    // Render box backgrounds FIRST (so participants appear on top)
    render_participant_box_backgrounds(f, area, world, &positions);

    // Then render individual participant boxes
    for (i, name) in diagram.participants.iter().enumerate() {
        let style = if selection == Selection::Participant(i) {
            theme.selected
        } else {
            theme.text
        };

        let width = name.len() as u16 + 4;
        let x = positions[i]
            .saturating_sub(width / 2)
            .min(area.width.saturating_sub(width));

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

fn render_participant_box_backgrounds(f: &mut Frame, area: Rect, world: &World, positions: &[u16]) {
    let diagram = world.get::<SequenceDiagram>();

    for b in &diagram.boxes {
        if b.start >= positions.len() || b.end >= positions.len() {
            continue;
        }

        let start_name = &diagram.participants[b.start];
        let end_name = &diagram.participants[b.end];

        let start_w = (start_name.len() as u16 + 4).min(area.width);
        let end_w = (end_name.len() as u16 + 4).min(area.width);

        let start_x = positions[b.start]
            .saturating_sub(start_w / 2)
            .min(area.width.saturating_sub(start_w));
        let end_x = positions[b.end]
            .saturating_sub(end_w / 2)
            .min(area.width.saturating_sub(end_w));

        let box_x = start_x.saturating_sub(1);
        let box_right = (end_x + end_w + 1).min(area.width);
        let box_width = box_right.saturating_sub(box_x);

        if box_width == 0 {
            continue;
        }

        let box_area = Rect {
            x: area.x + box_x,
            y: area.y,
            width: box_width,
            height: area.height,
        };

        let (bg_color, _label_color) = box_display_colors(b.color);

        // Borderless full-height background fill
        f.render_widget(
            Block::default().style(Style::default().bg(bg_color)),
            box_area,
        );
    }
}

fn render_box_labels(f: &mut Frame, area: Rect, world: &World, positions: &[u16]) {
    let diagram = world.get::<SequenceDiagram>();

    for b in &diagram.boxes {
        if b.label.is_empty() || b.start >= positions.len() || b.end >= positions.len() {
            continue;
        }

        let start_name = &diagram.participants[b.start];
        let end_name = &diagram.participants[b.end];

        let start_w = (start_name.len() as u16 + 4).min(area.width);
        let end_w = (end_name.len() as u16 + 4).min(area.width);

        let start_x = positions[b.start]
            .saturating_sub(start_w / 2)
            .min(area.width.saturating_sub(start_w));
        let end_x = positions[b.end]
            .saturating_sub(end_w / 2)
            .min(area.width.saturating_sub(end_w));

        let box_x = start_x.saturating_sub(1);
        let box_right = (end_x + end_w + 1).min(area.width);
        let box_width = box_right.saturating_sub(box_x);

        if box_width == 0 || area.height == 0 {
            continue;
        }

        let (bg_color, label_color) = box_display_colors(b.color);

        f.render_widget(
            Paragraph::new(b.label.as_str())
                .alignment(Alignment::Center)
                .style(Style::default().fg(label_color).bg(bg_color)),
            Rect {
                x: area.x + box_x,
                y: area.y + area.height - 1,
                width: box_width,
                height: 1,
            },
        );
    }
}

fn box_display_colors(color: BoxColor) -> (Color, Color) {
    match color {
        BoxColor::Blue => (Color::Rgb(20, 50, 100), Color::Rgb(100, 150, 220)),
        BoxColor::Green => (Color::Rgb(20, 70, 30), Color::Rgb(80, 180, 100)),
        BoxColor::Red => (Color::Rgb(100, 20, 20), Color::Rgb(220, 80, 80)),
        BoxColor::Yellow => (Color::Rgb(70, 60, 10), Color::Rgb(200, 180, 50)),
        BoxColor::Orange => (Color::Rgb(90, 45, 10), Color::Rgb(220, 130, 50)),
        BoxColor::Purple => (Color::Rgb(70, 20, 90), Color::Rgb(170, 80, 200)),
        BoxColor::Aqua => (Color::Rgb(10, 70, 80), Color::Rgb(60, 190, 200)),
        BoxColor::Gray => (Color::Rgb(45, 45, 45), Color::Rgb(150, 150, 150)),
    }
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

    let visible_range = scroll.visible_range(diagram);
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
    let visible_range = scroll.visible_range(diagram);

    let base_y = lifeline_start + FIRST_MESSAGE_OFFSET;
    let mut y = base_y;

    for (i, event) in diagram.events.iter().enumerate().skip(scroll.offset) {
        if !visible_range.contains(&i) {
            break;
        }

        let style = if selection == Selection::Event(i) {
            theme.selected
        } else {
            theme.text
        };

        match event {
            Event::Message { from, to, text } => {
                render_message(f, participants, *from, *to, text, y, style);
            }
            Event::Note {
                position,
                participant_start,
                participant_end,
                text,
            } => {
                render_note(
                    f,
                    participants,
                    *position,
                    *participant_start,
                    *participant_end,
                    text,
                    y,
                    style,
                );
            }
        }

        y += event.height();
    }
}

fn render_message(
    f: &mut Frame,
    participants: &[u16],
    from: usize,
    to: usize,
    text: &str,
    y: u16,
    style: ratatui::style::Style,
) {
    let from_x = participants[from];
    let to_x = participants[to];

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
        f.render_widget(Paragraph::new(text).style(style), area);
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
            Paragraph::new(text)
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

#[allow(clippy::too_many_arguments)]
fn render_note(
    f: &mut Frame,
    participants: &[u16],
    position: NotePosition,
    participant_start: usize,
    participant_end: usize,
    text: &str,
    y: u16,
    style: ratatui::style::Style,
) {
    let start_x = participants[participant_start];
    let end_x = participants[participant_end];
    let text_width = text.len() as u16;

    let (x, width) = match position {
        NotePosition::Right => (start_x.saturating_add(2), text_width + 4),
        NotePosition::Left => {
            let w = text_width + 4;
            (start_x.saturating_sub(w + 1), w)
        }
        NotePosition::Over => {
            let min_x = start_x.min(end_x);
            let max_x = start_x.max(end_x);
            let span = max_x.saturating_sub(min_x);
            let w = span.max(text_width + 2) + 2;
            let x = if span > 0 {
                min_x.saturating_sub(1)
            } else {
                min_x.saturating_sub(w / 2)
            };
            (x, w)
        }
    };

    let area = Rect {
        x,
        y: y.saturating_sub(1),
        width,
        height: 1,
    };

    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(text)
            .alignment(Alignment::Center)
            .style(style.reversed()),
        area,
    );
}
