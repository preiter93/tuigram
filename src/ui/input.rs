use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use tui_world::World;

use super::{EditorMode, EditorState};
use crate::{
    core::{NotePosition, SequenceDiagram},
    theme::Theme,
};

#[allow(clippy::too_many_lines)]
pub fn render_input_popup(frame: &mut Frame, world: &World) {
    let editor = world.get::<EditorState>();
    let diagram = world.get::<SequenceDiagram>();
    let theme = world.get::<Theme>();

    let (title, prompt) = match &editor.mode {
        EditorMode::InputParticipant => ("Add Participant".to_string(), Some("Name:".to_string())),
        EditorMode::RenameParticipant => {
            ("Rename Participant".to_string(), Some("Name:".to_string()))
        }
        EditorMode::InputMessage | EditorMode::EditMessage => {
            let from_name = editor
                .message_from
                .and_then(|i| diagram.participants.get(i))
                .map_or("?", String::as_str);
            let to_name = editor
                .message_to
                .and_then(|i| diagram.participants.get(i))
                .map_or("?", String::as_str);
            let title = if editor.mode == EditorMode::EditMessage {
                "Edit Message"
            } else {
                "Message"
            };
            (title.to_string(), Some(format!("{from_name} → {to_name}:")))
        }
        EditorMode::InputNoteText | EditorMode::EditNoteText => {
            let position = editor.note_position;
            let start_name = editor
                .note_participant_start
                .and_then(|i| diagram.participants.get(i))
                .map_or("?", String::as_str);
            let end_name = editor
                .note_participant_end
                .and_then(|i| diagram.participants.get(i))
                .map_or("?", String::as_str);

            let pos_str = match position {
                NotePosition::Right => format!("Right of {start_name}"),
                NotePosition::Left => format!("Left of {start_name}"),
                NotePosition::Over => {
                    if start_name == end_name {
                        format!("Over {start_name}")
                    } else {
                        format!("Over {start_name},{end_name}")
                    }
                }
            };

            let title = if editor.mode == EditorMode::EditNoteText {
                "Edit Note"
            } else {
                "Note"
            };
            (title.to_string(), Some(format!("{pos_str}:")))
        }
        _ => return,
    };

    let input = &editor.input_buffer;
    let area = frame.area();

    let popup_width = 50.min(area.width.saturating_sub(4));
    let popup_height = if prompt.is_some() { 5 } else { 4 };

    let popup_area = centered_rect(popup_width, popup_height, area);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(format!(" {title} "))
        .borders(Borders::ALL)
        .border_style(theme.border);

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let padding = 1u16;
    let mut input_y = inner.y;

    if let Some(prompt_text) = prompt {
        let prompt_area = Rect {
            x: inner.x + padding,
            y: inner.y,
            width: inner.width.saturating_sub(padding * 2),
            height: 1,
        };
        frame.render_widget(Paragraph::new(prompt_text).style(theme.muted), prompt_area);
        input_y = inner.y + 1;
    }

    let input_area = Rect {
        x: inner.x + padding,
        y: input_y,
        width: inner.width.saturating_sub(padding * 2),
        height: 1,
    };

    let input_line = Line::from(vec![
        Span::styled(input, theme.text),
        Span::styled("█", theme.accent),
    ]);
    frame.render_widget(Paragraph::new(input_line), input_area);

    let hint_area = Rect {
        x: inner.x + padding,
        y: input_y + 1,
        width: inner.width.saturating_sub(padding * 2),
        height: 1,
    };
    let hint_text = "Enter: confirm | Esc: cancel";
    frame.render_widget(
        Paragraph::new(hint_text)
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
