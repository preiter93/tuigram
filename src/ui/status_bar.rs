use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::{core::EditorMode, theme::Theme};

pub fn render_status_bar(
    frame: &mut Frame,
    area: Rect,
    mode: &EditorMode,
    _status_message: Option<&str>,
    participant_count: usize,
    has_selection: bool,
    theme: &Theme,
) {
    let (mode_text, mode_style) = match mode {
        EditorMode::Normal => ("NORMAL", theme.status_normal),
        EditorMode::InputParticipant => ("INPUT", theme.status_input),
        EditorMode::SelectFrom => ("SELECT FROM", theme.status_select),
        EditorMode::SelectTo => ("SELECT TO", theme.status_select),
        EditorMode::InputMessage => ("INPUT", theme.status_input),
        EditorMode::Help => ("HELP", theme.status_help),
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
        EditorMode::InputParticipant | EditorMode::InputMessage => "Enter: confirm  Esc: cancel",
        EditorMode::SelectFrom | EditorMode::SelectTo => "↑↓: navigate  Enter: select  Esc: cancel",
        EditorMode::Help => "?: close",
    };

    let line = Line::from(vec![
        Span::styled(format!(" {} ", mode_text), mode_style),
        Span::raw(" "),
        Span::styled(hints, theme.muted),
    ]);

    frame.render_widget(Paragraph::new(line), area);
}
