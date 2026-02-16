#![allow(clippy::too_many_lines)]

use crate::{
    core::SequenceDiagram,
    render::render_sequence,
    theme::Theme,
    ui::{EditorMode, EditorState, Selection},
    ui::{help::render_help, input::render_input_popup, status_bar::render_status_bar},
};
use ratatui::{
    Frame,
    crossterm::event::KeyCode,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use std::fs;
use tui_world::{KeyBinding, Keybindings, WidgetId, World};

pub const GLOBAL: WidgetId = WidgetId("Global");
pub const INPUT_MODE: WidgetId = WidgetId("InputMode");

#[derive(Default)]
pub struct AppState {
    pub should_quit: bool,
    pub area: Rect,
}

pub fn setup_world(world: &mut World, diagram: SequenceDiagram) {
    world.insert(Theme::default());
    world.insert(AppState::default());
    world.insert(diagram);
    world.insert(EditorState::new());

    global_keybindings(world);
    input_mode_keybindings(world);
}

fn global_keybindings(world: &mut World) {
    let kb = world.get_mut::<Keybindings>();

    kb.bind(GLOBAL, KeyBinding::ctrl('c'), "Quit", |world| {
        world.get_mut::<AppState>().should_quit = true;
    });

    kb.bind(GLOBAL, 'p', "Add participant", |world| {
        let mode = world.get::<EditorState>().mode.clone();
        if mode == EditorMode::Normal {
            let editor = world.get_mut::<EditorState>();
            editor.mode = EditorMode::InputParticipant;
            editor.input_buffer.clear();
        }
    });

    kb.bind(GLOBAL, 'e', "Add event", |world| {
        let mode = world.get::<EditorState>().mode.clone();
        if mode == EditorMode::Normal {
            let participant_count = world.get::<SequenceDiagram>().participant_count();
            if participant_count >= 2 {
                let editor = world.get_mut::<EditorState>();
                editor.mode = EditorMode::SelectFrom;
                editor.selected_index = 0;
                editor.message_from = None;
                editor.message_to = None;
            }
        }
    });

    kb.bind(GLOBAL, 'd', "Delete selected", |world| {
        let mode = world.get::<EditorState>().mode.clone();
        if mode != EditorMode::Normal {
            return;
        }

        let selection = world.get::<EditorState>().selection;
        match selection {
            Selection::Participant(idx) => {
                world.get_mut::<SequenceDiagram>().remove_participant(idx);
                let new_count = world.get::<SequenceDiagram>().participant_count();
                let editor = world.get_mut::<EditorState>();
                if new_count == 0 {
                    editor.clear_selection();
                } else {
                    editor.selection = Selection::Participant(idx.min(new_count - 1));
                }
            }
            Selection::Event(idx) => {
                world.get_mut::<SequenceDiagram>().remove_event(idx);
                let new_count = world.get::<SequenceDiagram>().event_count();
                let editor = world.get_mut::<EditorState>();
                if new_count == 0 {
                    editor.clear_selection();
                } else {
                    editor.selection = Selection::Event(idx.min(new_count - 1));
                }
            }
            Selection::None => {
                world.get_mut::<SequenceDiagram>().events.pop();
            }
        }
    });

    kb.bind(
        GLOBAL,
        KeyBinding::key(KeyCode::Tab),
        "Next item",
        |world| {
            let mode = world.get::<EditorState>().mode.clone();
            if mode != EditorMode::Normal {
                return;
            }

            let diagram = world.get::<SequenceDiagram>();
            let participant_count = diagram.participant_count();
            let event_count = diagram.event_count();

            let selection = world.get::<EditorState>().selection;
            let new_selection = selection.cycle(participant_count, event_count);
            world.get_mut::<EditorState>().selection = new_selection;
        },
    );

    kb.bind(
        GLOBAL,
        KeyBinding::new(
            KeyCode::BackTab,
            ratatui::crossterm::event::KeyModifiers::SHIFT,
        ),
        "Previous item",
        |world| {
            let mode = world.get::<EditorState>().mode.clone();
            if mode != EditorMode::Normal {
                return;
            }

            let diagram = world.get::<SequenceDiagram>();
            let participant_count = diagram.participant_count();
            let event_count = diagram.event_count();

            let selection = world.get::<EditorState>().selection;
            let new_selection = selection.cycle_back(participant_count, event_count);
            world.get_mut::<EditorState>().selection = new_selection;
        },
    );

    kb.bind(GLOBAL, 'H', "Move participant left", |world| {
        let mode = world.get::<EditorState>().mode.clone();
        if mode != EditorMode::Normal {
            return;
        }

        let selection = world.get::<EditorState>().selection;
        if let Selection::Participant(idx) = selection
            && idx > 0
        {
            world
                .get_mut::<SequenceDiagram>()
                .swap_participants(idx, idx - 1);
            world.get_mut::<EditorState>().selection = Selection::Participant(idx - 1);
        }
    });

    kb.bind(GLOBAL, 'L', "Move participant right", |world| {
        let mode = world.get::<EditorState>().mode.clone();
        if mode != EditorMode::Normal {
            return;
        }

        let selection = world.get::<EditorState>().selection;
        if let Selection::Participant(idx) = selection {
            let participant_count = world.get::<SequenceDiagram>().participant_count();
            if idx + 1 < participant_count {
                world
                    .get_mut::<SequenceDiagram>()
                    .swap_participants(idx, idx + 1);
                world.get_mut::<EditorState>().selection = Selection::Participant(idx + 1);
            }
        }
    });

    kb.bind(GLOBAL, 'J', "Move event down", |world| {
        let mode = world.get::<EditorState>().mode.clone();
        if mode != EditorMode::Normal {
            return;
        }

        let selection = world.get::<EditorState>().selection;
        if let Selection::Event(idx) = selection {
            let event_count = world.get::<SequenceDiagram>().event_count();
            if idx + 1 < event_count {
                world.get_mut::<SequenceDiagram>().events.swap(idx, idx + 1);
                world.get_mut::<EditorState>().selection = Selection::Event(idx + 1);
            }
        }
    });

    kb.bind(GLOBAL, 'K', "Move event up", |world| {
        let mode = world.get::<EditorState>().mode.clone();
        if mode != EditorMode::Normal {
            return;
        }

        let selection = world.get::<EditorState>().selection;
        if let Selection::Event(idx) = selection
            && idx > 0
        {
            world.get_mut::<SequenceDiagram>().events.swap(idx, idx - 1);
            world.get_mut::<EditorState>().selection = Selection::Event(idx - 1);
        }
    });

    kb.bind(GLOBAL, 'j', "Next event", |world| {
        let mode = world.get::<EditorState>().mode.clone();
        if mode != EditorMode::Normal {
            return;
        }

        let diagram = world.get::<SequenceDiagram>();
        let event_count = diagram.event_count();
        let participant_count = diagram.participant_count();

        let selection = world.get::<EditorState>().selection;
        world.get_mut::<EditorState>().selection =
            selection.next_event(event_count, participant_count);
    });

    kb.bind(GLOBAL, 'k', "Previous event", |world| {
        let mode = world.get::<EditorState>().mode.clone();
        if mode != EditorMode::Normal {
            return;
        }

        let diagram = world.get::<SequenceDiagram>();
        let event_count = diagram.event_count();
        let participant_count = diagram.participant_count();

        let selection = world.get::<EditorState>().selection;
        world.get_mut::<EditorState>().selection =
            selection.prev_event(event_count, participant_count);
    });

    kb.bind(GLOBAL, 'l', "Next participant", |world| {
        let mode = world.get::<EditorState>().mode.clone();
        if mode != EditorMode::Normal {
            return;
        }

        let diagram = world.get::<SequenceDiagram>();
        let participant_count = diagram.participant_count();
        let event_count = diagram.event_count();

        let selection = world.get::<EditorState>().selection;
        world.get_mut::<EditorState>().selection =
            selection.next_participant(participant_count, event_count);
    });

    kb.bind(GLOBAL, 'h', "Previous participant", |world| {
        let mode = world.get::<EditorState>().mode.clone();
        if mode != EditorMode::Normal {
            return;
        }

        let diagram = world.get::<SequenceDiagram>();
        let participant_count = diagram.participant_count();
        let event_count = diagram.event_count();

        let selection = world.get::<EditorState>().selection;
        world.get_mut::<EditorState>().selection =
            selection.prev_participant(participant_count, event_count);
    });

    kb.bind(GLOBAL, 'c', "Clear diagram", |world| {
        let mode = world.get::<EditorState>().mode.clone();
        if mode == EditorMode::Normal {
            let diagram = world.get_mut::<SequenceDiagram>();
            diagram.participants.clear();
            diagram.events.clear();
        }
    });

    kb.bind(GLOBAL, 'm', "Export Mermaid", |world| {
        let mode = world.get::<EditorState>().mode.clone();
        if mode == EditorMode::Normal {
            let diagram = world.get::<SequenceDiagram>();
            let mermaid = diagram.to_mermaid();
            match fs::write("diagram.mmd", &mermaid) {
                Ok(()) => world
                    .get_mut::<EditorState>()
                    .set_status("Exported to diagram.mmd"),
                Err(e) => world
                    .get_mut::<EditorState>()
                    .set_status(format!("Export failed: {e}")),
            }
        }
    });

    kb.bind(GLOBAL, '?', "Help", |world| {
        let editor = world.get_mut::<EditorState>();
        if editor.mode == EditorMode::Help {
            editor.mode = EditorMode::Normal;
        } else if editor.mode == EditorMode::Normal {
            editor.mode = EditorMode::Help;
        }
    });

    kb.bind(GLOBAL, KeyBinding::key(KeyCode::Esc), "Cancel", |world| {
        let editor = world.get_mut::<EditorState>();
        if editor.mode == EditorMode::Normal {
            editor.clear_selection();
        } else {
            editor.reset();
        }
    });
}

fn input_mode_keybindings(world: &mut World) {
    let kb = world.get_mut::<Keybindings>();

    kb.bind(
        INPUT_MODE,
        KeyBinding::key(KeyCode::Enter),
        "Confirm",
        |world| {
            let mode = world.get::<EditorState>().mode.clone();
            match mode {
                EditorMode::InputParticipant => {
                    let name = world.get::<EditorState>().input_buffer.trim().to_string();
                    if !name.is_empty() {
                        world.get_mut::<SequenceDiagram>().add_participant(name);
                    }
                    world.get_mut::<EditorState>().reset();
                }
                EditorMode::SelectFrom => {
                    let selected = world.get::<EditorState>().selected_index;
                    let editor = world.get_mut::<EditorState>();
                    editor.message_from = Some(selected);
                    editor.mode = EditorMode::SelectTo;
                    editor.selected_index = selected;
                }
                EditorMode::SelectTo => {
                    let selected = world.get::<EditorState>().selected_index;
                    let editor = world.get_mut::<EditorState>();
                    editor.message_to = Some(selected);
                    editor.mode = EditorMode::InputMessage;
                    editor.input_buffer.clear();
                }
                EditorMode::InputMessage => {
                    let editor_state = world.get::<EditorState>().clone();
                    let text = editor_state.input_buffer.trim().to_string();
                    if !text.is_empty()
                        && let (Some(from), Some(to)) =
                            (editor_state.message_from, editor_state.message_to)
                    {
                        world
                            .get_mut::<SequenceDiagram>()
                            .add_message(from, to, text);
                    }
                    world.get_mut::<EditorState>().reset();
                }
                _ => {}
            }
        },
    );

    kb.bind(
        INPUT_MODE,
        KeyBinding::key(KeyCode::Esc),
        "Cancel",
        |world| {
            world.get_mut::<EditorState>().reset();
        },
    );

    kb.bind(
        INPUT_MODE,
        KeyBinding::key(KeyCode::Backspace),
        "Delete",
        |world| {
            let mode = world.get::<EditorState>().mode.clone();
            if matches!(
                mode,
                EditorMode::InputParticipant | EditorMode::InputMessage
            ) {
                world.get_mut::<EditorState>().input_buffer.pop();
            }
        },
    );

    kb.bind(INPUT_MODE, KeyBinding::key(KeyCode::Up), "Up", |world| {
        handle_selection_nav(world, -1);
    });

    kb.bind(
        INPUT_MODE,
        KeyBinding::key(KeyCode::Down),
        "Down",
        |world| {
            handle_selection_nav(world, 1);
        },
    );

    kb.bind(
        INPUT_MODE,
        KeyBinding::key(KeyCode::Left),
        "Left",
        |world| {
            handle_selection_nav(world, -1);
        },
    );

    kb.bind(
        INPUT_MODE,
        KeyBinding::key(KeyCode::Right),
        "Right",
        |world| {
            handle_selection_nav(world, 1);
        },
    );

    kb.bind(INPUT_MODE, 'k', "Up", |world| {
        let mode = world.get::<EditorState>().mode.clone();
        if matches!(mode, EditorMode::SelectFrom | EditorMode::SelectTo) {
            handle_selection_nav(world, -1);
        } else if matches!(
            mode,
            EditorMode::InputParticipant | EditorMode::InputMessage
        ) {
            world.get_mut::<EditorState>().input_buffer.push('k');
        }
    });

    kb.bind(INPUT_MODE, 'j', "Down", |world| {
        let mode = world.get::<EditorState>().mode.clone();
        if matches!(mode, EditorMode::SelectFrom | EditorMode::SelectTo) {
            handle_selection_nav(world, 1);
        } else if matches!(
            mode,
            EditorMode::InputParticipant | EditorMode::InputMessage
        ) {
            world.get_mut::<EditorState>().input_buffer.push('j');
        }
    });

    kb.bind(INPUT_MODE, 'h', "Left", |world| {
        let mode = world.get::<EditorState>().mode.clone();
        if matches!(mode, EditorMode::SelectFrom | EditorMode::SelectTo) {
            handle_selection_nav(world, -1);
        } else if matches!(
            mode,
            EditorMode::InputParticipant | EditorMode::InputMessage
        ) {
            world.get_mut::<EditorState>().input_buffer.push('h');
        }
    });

    kb.bind(INPUT_MODE, 'l', "Right", |world| {
        let mode = world.get::<EditorState>().mode.clone();
        if matches!(mode, EditorMode::SelectFrom | EditorMode::SelectTo) {
            handle_selection_nav(world, 1);
        } else if matches!(
            mode,
            EditorMode::InputParticipant | EditorMode::InputMessage
        ) {
            world.get_mut::<EditorState>().input_buffer.push('l');
        }
    });

    for digit in '1'..='9' {
        kb.bind(INPUT_MODE, digit, "Select", move |world| {
            let mode = world.get::<EditorState>().mode.clone();
            let participant_count = world.get::<SequenceDiagram>().participant_count();
            let num = digit.to_digit(10).unwrap() as usize;

            if matches!(mode, EditorMode::SelectFrom | EditorMode::SelectTo) {
                if num >= 1 && num <= participant_count {
                    world.get_mut::<EditorState>().selected_index = num - 1;
                }
            } else if matches!(
                mode,
                EditorMode::InputParticipant | EditorMode::InputMessage
            ) {
                world.get_mut::<EditorState>().input_buffer.push(digit);
            }
        });
    }

    kb.bind_any(INPUT_MODE, |world, key| {
        let mode = world.get::<EditorState>().mode.clone();
        if matches!(
            mode,
            EditorMode::InputParticipant | EditorMode::InputMessage
        ) && let KeyCode::Char(c) = key.code
        {
            world.get_mut::<EditorState>().input_buffer.push(c);
        }
    });
}

#[allow(
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn handle_selection_nav(world: &mut World, delta: i32) {
    let mode = world.get::<EditorState>().mode.clone();
    if !matches!(mode, EditorMode::SelectFrom | EditorMode::SelectTo) {
        return;
    }

    let participant_count = world.get::<SequenceDiagram>().participant_count();
    if participant_count == 0 {
        return;
    }

    let editor = world.get_mut::<EditorState>();
    let current = editor.selected_index as i32;
    let new_idx = (current + delta).rem_euclid(participant_count as i32) as usize;
    editor.selected_index = new_idx;
}

pub fn active_widgets(world: &World) -> Vec<WidgetId> {
    let mode = world.get::<EditorState>().mode.clone();
    match mode {
        EditorMode::Normal | EditorMode::Help => vec![GLOBAL],
        _ => vec![INPUT_MODE],
    }
}

pub fn render(frame: &mut Frame, world: &mut World) {
    let area = frame.area();
    world.get_mut::<AppState>().area = area;

    let [diagram_area, status_area] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(area);

    let diagram = world.get::<SequenceDiagram>();
    let editor = world.get::<EditorState>();
    let theme = world.get::<Theme>();
    let keybindings = world.get::<Keybindings>();

    let selection = editor.selection;

    if diagram.participants.is_empty() {
        render_empty_state(frame, diagram_area, theme);
    } else {
        render_sequence(frame, diagram_area, diagram, selection, theme);
    }

    let has_selection = selection != Selection::None;

    render_status_bar(
        frame,
        status_area,
        &editor.mode,
        editor.get_status(),
        diagram.participant_count(),
        has_selection,
        theme,
    );

    match &editor.mode {
        EditorMode::InputParticipant => {
            render_input_popup(
                frame,
                "Add Participant",
                &editor.input_buffer,
                "Name:",
                theme,
            );
        }
        EditorMode::SelectFrom | EditorMode::SelectTo => {
            render_dual_participant_selector(
                frame,
                area,
                &diagram.participants,
                editor.message_from,
                editor.selected_index,
                editor.mode == EditorMode::SelectFrom,
                theme,
            );
        }
        EditorMode::InputMessage => {
            let from_name = editor
                .message_from
                .and_then(|i| diagram.participants.get(i))
                .map_or("?", String::as_str);
            let to_name = editor
                .message_to
                .and_then(|i| diagram.participants.get(i))
                .map_or("?", String::as_str);
            let prompt = format!("{from_name} → {to_name}:");
            render_input_popup(frame, "Message", &editor.input_buffer, &prompt, theme);
        }
        EditorMode::Help => {
            let active = vec![GLOBAL];
            render_help(frame, area, theme, keybindings, &active);
        }
        EditorMode::Normal => {}
    }
}

#[allow(clippy::cast_possible_truncation)]
fn render_empty_state(frame: &mut Frame, area: Rect, theme: &Theme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let logo = vec![
        Line::from(Span::styled("╔╦╗╦ ╦╦╔═╗╦═╗╔═╗╔╦╗", theme.accent)),
        Line::from(Span::styled(" ║ ║ ║║║ ╦╠╦╝╠═╣║║║", theme.accent)),
        Line::from(Span::styled(" ╩ ╚═╝╩╚═╝╩╚═╩ ╩╩ ╩", theme.accent)),
    ];

    let subtitle = Line::from(Span::styled("Sequence Diagram Editor", theme.muted));

    let keybinds = vec![
        Line::from(vec![
            Span::styled("p", theme.key),
            Span::styled("  Add participant", theme.text),
        ]),
        Line::from(vec![
            Span::styled("?", theme.key),
            Span::styled("  Show help", theme.text),
        ]),
        Line::from(vec![
            Span::styled("Ctrl+c", theme.key),
            Span::styled("  Quit", theme.text),
        ]),
    ];

    let total_height = logo.len() + 2 + keybinds.len() + 2;

    let [_, content_area, _] = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(total_height as u16),
        Constraint::Min(0),
    ])
    .flex(Flex::Center)
    .areas(inner);

    let [logo_area, _, subtitle_area, _, keys_area] = Layout::vertical([
        Constraint::Length(logo.len() as u16),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(keybinds.len() as u16),
    ])
    .areas(content_area);

    frame.render_widget(Paragraph::new(logo).alignment(Alignment::Center), logo_area);
    frame.render_widget(
        Paragraph::new(subtitle).alignment(Alignment::Center),
        subtitle_area,
    );
    frame.render_widget(
        Paragraph::new(keybinds).alignment(Alignment::Center),
        keys_area,
    );
}

#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::too_many_arguments)]
fn render_dual_participant_selector(
    frame: &mut Frame,
    area: Rect,
    participants: &[String],
    from_idx: Option<usize>,
    cursor: usize,
    selecting_from: bool,
    theme: &Theme,
) {
    let popup_width = 50.min(area.width.saturating_sub(4));
    let popup_height = (participants.len() as u16 + 4).min(area.height.saturating_sub(4));

    let popup_area = centered_rect(popup_width, popup_height, area);

    frame.render_widget(ratatui::widgets::Clear, popup_area);

    let title = if selecting_from { " From " } else { " To " };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(theme.border);

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let col_width = inner.width / 2;

    for (i, name) in participants.iter().enumerate() {
        if i as u16 >= inner.height {
            break;
        }

        let y = inner.y + i as u16;

        let from_selected = from_idx == Some(i);
        let from_cursor = selecting_from && cursor == i;
        let from_prefix = if from_cursor || from_selected {
            "▶ "
        } else {
            "  "
        };
        let from_style = if selecting_from {
            if from_cursor || from_selected {
                theme.selected
            } else {
                theme.text
            }
        } else {
            theme.muted
        };
        frame.render_widget(
            Paragraph::new(format!("{from_prefix}{name}")).style(from_style),
            Rect {
                x: inner.x,
                y,
                width: col_width,
                height: 1,
            },
        );

        let to_cursor = !selecting_from && cursor == i;
        let to_prefix = if to_cursor { "▶ " } else { "  " };
        let to_style = if selecting_from {
            theme.muted
        } else if to_cursor {
            theme.selected
        } else {
            theme.text
        };
        frame.render_widget(
            Paragraph::new(format!("{to_prefix}{name}")).style(to_style),
            Rect {
                x: inner.x + col_width,
                y,
                width: col_width,
                height: 1,
            },
        );
    }
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
