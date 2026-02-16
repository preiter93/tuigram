#![allow(clippy::cast_possible_truncation)]

use ratatui::{
    Frame,
    layout::{Alignment, Margin, Rect},
    text::Line,
    widgets::{Block, Borders, Paragraph},
};

use crate::{
    core::{Event, SequenceDiagram},
    theme::Theme,
    ui::state::Selection,
};

pub struct ParticipantLayout {
    pub index: usize,
    pub name: String,
    pub x: u16,
}

pub struct MessageLayout {
    pub index: usize,
    pub from_x: u16,
    pub to_x: u16,
    pub y: u16,
    pub text: String,
}

pub struct SequenceLayout {
    pub width: u16,
    pub height: u16,
    pub participants: Vec<ParticipantLayout>,
    pub messages: Vec<MessageLayout>,
}

impl SequenceLayout {
    pub fn compute(diagram: &SequenceDiagram, term_width: u16) -> Self {
        let header_height = 3;
        let mut current_y = header_height;

        let participants = {
            let mut participants = Vec::new();
            let count = diagram.participants.len().max(1) as u16;
            let spacing = term_width / (count + 1);

            for (i, name) in diagram.participants.iter().enumerate() {
                let x = spacing * (i as u16 + 1);
                participants.push(ParticipantLayout {
                    index: i,
                    name: name.clone(),
                    x,
                });
            }

            participants
        };

        let messages = {
            let mut messages = Vec::new();
            let message_spacing = 3;

            for (i, event) in diagram.events.iter().enumerate() {
                let Event::Message { from, to, text } = event;
                let from_x = participants[*from].x;
                let to_x = participants[*to].x;

                messages.push(MessageLayout {
                    index: i,
                    from_x,
                    to_x,
                    y: current_y,
                    text: text.clone(),
                });

                current_y += message_spacing;
            }

            messages
        };

        Self {
            width: term_width,
            height: current_y + 2,
            participants,
            messages,
        }
    }
}

fn render_participants(
    f: &mut Frame,
    area: Rect,
    layout: &SequenceLayout,
    selection: Selection,
    theme: &Theme,
) {
    for p in &layout.participants {
        let is_selected = matches!(selection, Selection::Participant(i) if i == p.index);
        let width = (p.name.len() as u16).saturating_add(4);
        let x =
            p.x.saturating_sub(width / 2)
                .min(area.width.saturating_sub(width));
        let participant_area = Rect {
            x,
            y: area.y,
            width,
            height: 3,
        };

        let style = if is_selected {
            theme.selected
        } else {
            theme.text
        };

        let block = Block::default().borders(Borders::ALL).border_style(style);
        f.render_widget(block, participant_area);

        let paragraph = Paragraph::new(p.name.as_str())
            .alignment(Alignment::Center)
            .style(style);
        let inner_area = Rect {
            x: participant_area.x + 1,
            y: participant_area.y + 1,
            width: participant_area.width.saturating_sub(2),
            height: participant_area.height.saturating_sub(2),
        };
        f.render_widget(paragraph, inner_area);
    }
}

fn render_lifelines(f: &mut Frame, area: Rect, layout: &SequenceLayout, theme: &Theme) {
    let start_y = area.y + 3;
    for p in &layout.participants {
        let xi = p.x.min(area.width - 1);
        for y in start_y..area.height + area.y {
            let line_area = Rect {
                x: xi,
                y,
                width: 1,
                height: 1,
            };
            f.render_widget(Paragraph::new(Line::from("│")).style(theme.text), line_area);
        }
    }
}

fn render_messages(
    f: &mut Frame,
    area: Rect,
    layout: &SequenceLayout,
    selection: Selection,
    theme: &Theme,
) {
    let start_y = area.y + 3;
    for msg in &layout.messages {
        let is_selected = matches!(selection, Selection::Event(i) if i == msg.index);
        let y = start_y + msg.y;

        let start = msg.from_x.min(msg.to_x);
        let end = msg.from_x.max(msg.to_x);
        let arrow_len = end - start;

        let line_area = Rect {
            x: start,
            y,
            width: arrow_len + 1,
            height: 1,
        };

        let mut line = "─".repeat(arrow_len as usize);
        if msg.from_x < msg.to_x {
            line.push('>');
        } else {
            line.insert(0, '<');
        }

        let style = if is_selected {
            theme.selected
        } else {
            theme.text
        };

        f.render_widget(Paragraph::new(Line::from(line)).style(style), line_area);

        if y > start_y {
            let arrow_width = arrow_len + 1;
            let text_width = msg.text.len() as u16;
            let text_x = start + arrow_width / 2;
            let text_start = text_x
                .saturating_sub(text_width / 2)
                .min(start + arrow_width.saturating_sub(text_width));

            let text_area = Rect {
                x: text_start,
                y: y - 1,
                width: text_width,
                height: 1,
            };

            f.render_widget(
                Paragraph::new(msg.text.as_str())
                    .alignment(Alignment::Center)
                    .style(style),
                text_area,
            );
        }
    }
}

pub fn render_sequence(
    f: &mut Frame,
    outer_area: Rect,
    layout: &SequenceLayout,
    selection: Selection,
    theme: &Theme,
) {
    let diagram_block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border);
    f.render_widget(diagram_block, outer_area);

    let area = outer_area.inner(Margin::new(0, 1));

    render_participants(f, area, layout, selection, theme);
    render_lifelines(f, area, layout, theme);
    render_messages(f, area, layout, selection, theme);
}
