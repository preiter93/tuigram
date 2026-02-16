use ratatui::{
    Frame,
    layout::{Alignment, Margin, Rect},
    text::Line,
    widgets::{Block, Borders, Paragraph},
};

use crate::layout::sequence::SequenceLayout;

fn render_participants(f: &mut Frame, area: Rect, layout: &SequenceLayout) {
    for p in &layout.participants {
        let width = (p.name.len() as u16).saturating_add(4); // padding left/right
        let x =
            p.x.saturating_sub(width / 2)
                .min(area.width.saturating_sub(width));
        let participant_area = Rect {
            x,
            y: area.y,
            width,
            height: 3, // 3 rows high
        };

        // Draw the box
        let block = Block::default().borders(Borders::ALL);
        f.render_widget(block, participant_area);

        // Draw the name inside, centered
        let paragraph = Paragraph::new(p.name.as_str()).alignment(Alignment::Center);
        // Shrink the area to the inside of the borders
        let inner_area = Rect {
            x: participant_area.x + 1,
            y: participant_area.y + 1,
            width: participant_area.width.saturating_sub(2),
            height: participant_area.height.saturating_sub(2),
        };
        f.render_widget(paragraph, inner_area);
    }
}

/// Render vertical lifelines under each participant block
fn render_lifelines(f: &mut Frame, area: Rect, layout: &SequenceLayout) {
    let start_y = area.y + 3; // below participant blocks
    for p in &layout.participants {
        let xi = p.x.min(area.width - 1);
        for y in start_y..area.height + area.y {
            let line_area = Rect {
                x: xi,
                y,
                width: 1,
                height: 1,
            };
            let line = Paragraph::new(Line::from("│"));
            f.render_widget(line, line_area);
        }
    }
}

/// Render horizontal arrows/messages
fn render_messages(f: &mut Frame, area: Rect, layout: &SequenceLayout) {
    let start_y = area.y + 3; // below participant blocks
    for msg in &layout.messages {
        let y = start_y + msg.y;

        let start = msg.from_x.min(msg.to_x);
        let end = msg.from_x.max(msg.to_x);
        let arrow_len = end - start;

        // Horizontal line
        let line_area = Rect {
            x: start,
            y,
            width: arrow_len + 1,
            height: 1,
        };

        let mut line = String::from("─".repeat(arrow_len as usize));
        if msg.from_x < msg.to_x {
            line.push('>');
        } else {
            line.insert(0, '<');
        }

        let paragraph = Paragraph::new(Line::from(line));
        f.render_widget(paragraph, line_area);

        // Message text above arrow
        if y > start_y {
            let arrow_width = arrow_len + 1;
            let text_width = msg.text.len() as u16;
            let text_x = start + arrow_width / 2;
            let text_start = text_x.saturating_sub(text_width / 2);

            // Ensure we don’t overflow the arrow
            let text_start = text_start.min(start + arrow_width.saturating_sub(text_width));

            let text_area = Rect {
                x: text_start,
                y: y - 1,
                width: text_width,
                height: 1,
            };

            let paragraph =
                Paragraph::new(msg.text.as_str()).alignment(ratatui::layout::Alignment::Center);

            f.render_widget(paragraph, text_area);
        }
    }
}

/// Top-level render
pub fn render_sequence(f: &mut Frame, outer_area: Rect, layout: &SequenceLayout) {
    // Outer block
    let diagram_block = Block::default().borders(Borders::ALL);
    f.render_widget(diagram_block, outer_area);

    let area = outer_area.inner(Margin::new(0, 1));

    render_participants(f, area, layout);
    render_lifelines(f, area, layout);
    render_messages(f, area, layout);
}
