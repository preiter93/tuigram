use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
};
use tui_world::World;

use super::{EditorMode, EditorState, Selection};
use crate::{core::SequenceDiagram, theme::Theme};

const DIRTY_HINTS: &str = "S: save  u: undo  ";

pub fn render_status_bar(
    frame: &mut Frame,
    area: Rect,
    world: &World,
    file_name: &str,
    dirty: bool,
) {
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
        EditorMode::EditMessage | EditorMode::RenameParticipant => ("EDIT", theme.status_input),
        EditorMode::Help => ("HELP", theme.status_help),
        EditorMode::ConfirmClear | EditorMode::ConfirmQuit => ("CONFIRM", theme.status_select),
        EditorMode::SelectNoteParticipant | EditorMode::EditNoteParticipant => {
            ("SELECT PARTICIPANT", theme.status_select)
        }
        EditorMode::SelectNotePosition | EditorMode::EditNotePosition => {
            ("SELECT POSITION", theme.status_select)
        }
        EditorMode::SelectNoteEndParticipant | EditorMode::EditNoteEndParticipant => {
            ("SELECT END", theme.status_select)
        }
        EditorMode::InputNoteText | EditorMode::EditNoteText => ("NOTE", theme.status_input),
        EditorMode::SelectBoxEnd => ("BOX END", theme.status_select),
        EditorMode::SelectBoxColor => ("BOX COLOR", theme.status_select),
        EditorMode::InputBoxLabel => ("BOX LABEL", theme.status_input),
        EditorMode::SelectFrameEnd => ("FRAME END", theme.status_select),
        EditorMode::SelectFrameKind => ("FRAME KIND", theme.status_select),
        EditorMode::InputFrameLabel => ("FRAME", theme.status_input),
        EditorMode::InputSectionLabel => ("SECTION", theme.status_input),
        EditorMode::InputSavePath => ("SAVE", theme.status_input),
    };

    let hints = match mode {
        EditorMode::Normal if matches!(editor.selection, Selection::Box(_)) => {
            "H/L: move  Enter: edit  D: remove box  p: participant  ?: help"
        }
        EditorMode::Normal if matches!(editor.selection, Selection::Fragment(_)) => {
            "J/K: move  Enter: edit  D: remove frame  m/M: message  ?: help"
        }
        EditorMode::Normal if matches!(editor.selection, Selection::Section(..)) => {
            "J/K: move  Enter: edit  D: remove section  m/M: message  ?: help"
        }
        EditorMode::Normal => {
            if participant_count < 2 {
                if has_selection {
                    "p: participant  D: delete  ?: help  Ctrl+c: quit"
                } else {
                    "p: participant  ?: help  Ctrl+c: quit"
                }
            } else if has_selection {
                "p: participant  m: message  n: note  b: box  D: delete  ?: help  Ctrl+c: quit"
            } else {
                "p: participant  m: message  n: note  b: box  ?: help  Ctrl+c: quit"
            }
        }
        EditorMode::InputParticipant
        | EditorMode::RenameParticipant
        | EditorMode::InputBoxLabel
        | EditorMode::InputFrameLabel
        | EditorMode::InputSectionLabel
        | EditorMode::InputSavePath => "Enter: confirm  Esc: cancel",
        EditorMode::InputMessage
        | EditorMode::EditMessage
        | EditorMode::InputNoteText
        | EditorMode::EditNoteText => "Enter: confirm  Alt+Enter: line break  Esc: cancel",
        EditorMode::SelectFrom
        | EditorMode::SelectTo
        | EditorMode::EditSelectFrom
        | EditorMode::EditSelectTo
        | EditorMode::SelectNoteParticipant
        | EditorMode::SelectNoteEndParticipant
        | EditorMode::EditNoteParticipant
        | EditorMode::EditNoteEndParticipant => "↑↓: navigate  Enter: select  Esc: cancel",
        EditorMode::SelectBoxEnd => "h/l: choose last participant  Enter: pick color  Esc: cancel",
        EditorMode::SelectNotePosition | EditorMode::EditNotePosition => {
            "↑↓: change position  Enter: confirm  Esc: cancel"
        }
        EditorMode::SelectBoxColor => "↑↓: change color  Enter: next (label)  Esc: cancel",
        EditorMode::SelectFrameEnd => "j/k: choose last message  Enter: pick kind  Esc: cancel",
        EditorMode::SelectFrameKind => "j/k: kind  Enter: next (label)  Esc: cancel",
        EditorMode::Help => "any key: close",
        EditorMode::ConfirmClear | EditorMode::ConfirmQuit => "y/Enter: confirm  n/Esc: cancel",
    };

    let mut spans = vec![
        Span::styled(format!(" {mode_text} "), mode_style),
        Span::raw(" "),
        Span::styled(file_name.to_string(), theme.text),
    ];

    if dirty {
        spans.push(Span::styled(" [+]", theme.accent));
    }

    spans.push(Span::raw("  "));

    if let Some(status) = status_message {
        let style = if status.is_error {
            theme.error
        } else {
            theme.success
        };
        spans.push(Span::styled(status.text.as_str(), style));
        spans.push(Span::raw("  "));
    }

    let used_width: usize = spans.iter().map(Span::width).sum();
    let hints = if used_width + hints.chars().count() > usize::from(area.width)
        && mode.is_message_text_input()
    {
        "Enter: confirm  Esc: cancel"
    } else {
        hints
    };
    let fits = used_width + DIRTY_HINTS.len() + hints.chars().count() <= usize::from(area.width);

    if dirty && *mode == EditorMode::Normal && fits {
        spans.push(Span::styled(DIRTY_HINTS, theme.muted));
    }

    if let Some(warning) = overlap_warning(editor, diagram) {
        spans.push(Span::styled(warning, theme.error));
        spans.push(Span::raw("  "));
    }

    spans.push(Span::styled(hints, theme.muted));

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(line), area);
}

/// Shown while the frame or box being created would be rejected.
fn overlap_warning(editor: &EditorState, diagram: &SequenceDiagram) -> Option<&'static str> {
    if let Some(range) = editor.frame_selection_range()
        && diagram.fragment_overlaps(*range.start(), *range.end())
    {
        return Some("overlaps a frame");
    }
    if let Some(range) = editor.box_selection_range()
        && diagram.box_overlaps(*range.start(), *range.end())
    {
        return Some("overlaps a box");
    }
    None
}
