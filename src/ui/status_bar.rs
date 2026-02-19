use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
};
use tui_world::World;

use super::{EditorMode, EditorState, Selection};
use crate::{core::SequenceDiagram, theme::Theme};

pub fn render_status_bar(frame: &mut Frame, area: Rect, world: &World) {
    let editor = world.get::<EditorState>();
    let diagram = world.get::<SequenceDiagram>();
    let theme = world.get::<Theme>();

    let mode = &editor.mode;
    let status_message = editor.get_status();
    let participant_count = diagram.participant_count();
    let has_selection = editor.selection != Selection::None;

    let (mode_text, mode_style) = match mode {
        EditorMode::Normal => ("NORMAL", theme.status_normal),
        EditorMode::InputParticipant | EditorMode::InputMessage => ("INPUT", theme.status_input),
        EditorMode::SelectFrom | EditorMode::EditSelectFrom => ("SELECT FROM", theme.status_select),
        EditorMode::SelectTo | EditorMode::EditSelectTo => ("SELECT TO", theme.status_select),
        EditorMode::EditMessage => ("EDIT", theme.status_input),
        EditorMode::Help => ("HELP", theme.status_help),
        EditorMode::ConfirmClear => ("CONFIRM", theme.status_select),
    };

    let hints = match mode {
        EditorMode::Normal => {
            if participant_count < 2 {
                if has_selection {
                    "p: participant  d: delete  ?: help  Ctrl+c: quit"
                } else {
                    "p: participant  ?: help  Ctrl+c: quit"
                }
            } else if has_selection {
                "p: participant  e: event  d: delete  ?: help  Ctrl+c: quit"
            } else {
                "p: participant  e: event  ?: help  Ctrl+c: quit"
            }
        }
        EditorMode::InputParticipant | EditorMode::InputMessage | EditorMode::EditMessage => {
            "Enter: confirm  Esc: cancel"
        }
        EditorMode::SelectFrom
        | EditorMode::SelectTo
        | EditorMode::EditSelectFrom
        | EditorMode::EditSelectTo => "↑↓: navigate  Enter: select  Esc: cancel",
        EditorMode::Help => "?: close",
        EditorMode::ConfirmClear => "y/Enter: confirm  n/Esc: cancel",
    };

    let mut spans = vec![
        Span::styled(format!(" {mode_text} "), mode_style),
        Span::raw(" "),
    ];

    if let Some(msg) = status_message {
        spans.push(Span::styled(msg, theme.success));
        spans.push(Span::raw("  "));
    }

    spans.push(Span::styled(hints, theme.muted));

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(line), area);
}
