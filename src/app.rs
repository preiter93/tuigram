#![allow(clippy::too_many_lines)]

use crate::{
    core::{Event, NotePosition, SequenceDiagram},
    render::render_sequence,
    theme::Theme,
    ui::{
        EditorMode, EditorState, Selection, confirm::render_confirm_dialog, help::render_help,
        input::render_input_popup, scroll::ScrollState, status_bar::render_status_bar,
    },
};
use ratatui::{
    Frame,
    crossterm::event::KeyCode,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use std::fs;
use tui_world::keys;
use tui_world::{KeyBinding, Keybindings, WidgetId, World};

pub const GLOBAL: WidgetId = WidgetId("Global");
pub const INPUT_MODE: WidgetId = WidgetId("InputMode");
pub const CONFIRM_MODE: WidgetId = WidgetId("ConfirmMode");

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
    world.insert(ScrollState::new());

    global_keybindings(world);
    input_mode_keybindings(world);
    confirm_mode_keybindings(world);
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

    kb.bind(GLOBAL, 'm', "Insert message after", |world| {
        let mode = world.get::<EditorState>().mode.clone();
        if mode == EditorMode::Normal {
            let participant_count = world.get::<SequenceDiagram>().participant_count();
            if participant_count >= 2 {
                let selection = world.get::<EditorState>().selection;
                let insert_after = match selection {
                    Selection::Event(idx) => Some(idx),
                    _ => None,
                };
                let editor = world.get_mut::<EditorState>();
                editor.mode = EditorMode::SelectFrom;
                editor.selected_index = 0;
                editor.message_from = None;
                editor.message_to = None;
                editor.insert_after_index = insert_after;
            }
        }
    });

    kb.bind(GLOBAL, 'M', "Insert message before", |world| {
        let mode = world.get::<EditorState>().mode.clone();
        if mode == EditorMode::Normal {
            let participant_count = world.get::<SequenceDiagram>().participant_count();
            if participant_count >= 2 {
                let selection = world.get::<EditorState>().selection;
                let insert_after = match selection {
                    Selection::Event(idx) if idx > 0 => Some(idx - 1),
                    Selection::Event(0) => Some(usize::MAX), // marker for insert at start
                    _ => None,
                };
                let editor = world.get_mut::<EditorState>();
                editor.mode = EditorMode::SelectFrom;
                editor.selected_index = 0;
                editor.message_from = None;
                editor.message_to = None;
                editor.insert_after_index = insert_after;
            }
        }
    });

    kb.bind(GLOBAL, 'n', "Insert note after", |world| {
        let mode = world.get::<EditorState>().mode.clone();
        if mode == EditorMode::Normal {
            let participant_count = world.get::<SequenceDiagram>().participant_count();
            if participant_count >= 1 {
                let selection = world.get::<EditorState>().selection;
                let insert_after = match selection {
                    Selection::Event(idx) => Some(idx),
                    _ => None,
                };
                let editor = world.get_mut::<EditorState>();
                editor.mode = EditorMode::SelectNoteParticipant;
                editor.selected_index = 0;
                editor.note_position = NotePosition::Right;
                editor.note_participant_start = None;
                editor.note_participant_end = None;
                editor.insert_after_index = insert_after;
            }
        }
    });

    kb.bind(GLOBAL, 'N', "Insert note before", |world| {
        let mode = world.get::<EditorState>().mode.clone();
        if mode == EditorMode::Normal {
            let participant_count = world.get::<SequenceDiagram>().participant_count();
            if participant_count >= 1 {
                let selection = world.get::<EditorState>().selection;
                let insert_after = match selection {
                    Selection::Event(idx) if idx > 0 => Some(idx - 1),
                    Selection::Event(0) => Some(usize::MAX),
                    _ => None,
                };
                let editor = world.get_mut::<EditorState>();
                editor.mode = EditorMode::SelectNoteParticipant;
                editor.selected_index = 0;
                editor.note_position = NotePosition::Right;
                editor.note_participant_start = None;
                editor.note_participant_end = None;
                editor.insert_after_index = insert_after;
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

    kb.bind_many(
        GLOBAL,
        keys!['l', KeyCode::Right],
        "Select Right",
        |world| {
            let mode = world.get::<EditorState>().mode.clone();
            if mode != EditorMode::Normal {
                return;
            }

            let participant_count = world.get::<SequenceDiagram>().participant_count();
            let selection = world.get::<EditorState>().selection;
            world.get_mut::<EditorState>().selection = selection.right(participant_count);
        },
    );

    kb.bind_many(GLOBAL, keys!['h', KeyCode::Left], "Select Left", |world| {
        let mode = world.get::<EditorState>().mode.clone();
        if mode != EditorMode::Normal {
            return;
        }

        let participant_count = world.get::<SequenceDiagram>().participant_count();
        let selection = world.get::<EditorState>().selection;
        world.get_mut::<EditorState>().selection = selection.left(participant_count);
    });

    kb.bind_many(GLOBAL, keys!['j', KeyCode::Down], "Select Down", |world| {
        let mode = world.get::<EditorState>().mode.clone();
        if mode != EditorMode::Normal {
            return;
        }

        let diagram = world.get::<SequenceDiagram>();
        let participant_count = diagram.participant_count();
        let event_count = diagram.event_count();
        let selection = world.get::<EditorState>().selection;

        // Remember participant index before moving to events
        if let Selection::Participant(idx) = selection {
            world.get_mut::<EditorState>().last_participant_index = Some(idx);
        }

        world.get_mut::<EditorState>().selection = selection.down(participant_count, event_count);
    });

    kb.bind_many(GLOBAL, keys!['k', KeyCode::Up], "Select Up", |world| {
        let mode = world.get::<EditorState>().mode.clone();
        if mode != EditorMode::Normal {
            return;
        }

        let participant_count = world.get::<SequenceDiagram>().participant_count();
        let editor = world.get::<EditorState>();
        let selection = editor.selection;
        let last_participant = editor.last_participant_index;

        // When going up from first event, restore last participant
        if let Selection::Event(0) = selection
            && let Some(idx) = last_participant
            && idx < participant_count
        {
            world.get_mut::<EditorState>().selection = Selection::Participant(idx);
            return;
        }

        world.get_mut::<EditorState>().selection = selection.up(participant_count);
    });

    kb.bind(
        GLOBAL,
        'H',
        "Move participant left / Point arrow left",
        |world| {
            let mode = world.get::<EditorState>().mode.clone();
            if mode != EditorMode::Normal {
                return;
            }

            let selection = world.get::<EditorState>().selection;
            match selection {
                Selection::Participant(idx) if idx > 0 => {
                    world
                        .get_mut::<SequenceDiagram>()
                        .swap_participants(idx, idx - 1);
                    world.get_mut::<EditorState>().selection = Selection::Participant(idx - 1);
                }
                Selection::Event(idx) => {
                    world.get_mut::<SequenceDiagram>().point_event_left(idx);
                }
                _ => {}
            }
        },
    );

    kb.bind(
        GLOBAL,
        'L',
        "Move participant right / Point arrow right",
        |world| {
            let mode = world.get::<EditorState>().mode.clone();
            if mode != EditorMode::Normal {
                return;
            }

            let selection = world.get::<EditorState>().selection;
            match selection {
                Selection::Participant(idx) => {
                    let participant_count = world.get::<SequenceDiagram>().participant_count();
                    if idx + 1 < participant_count {
                        world
                            .get_mut::<SequenceDiagram>()
                            .swap_participants(idx, idx + 1);
                        world.get_mut::<EditorState>().selection = Selection::Participant(idx + 1);
                    }
                }
                Selection::Event(idx) => {
                    world.get_mut::<SequenceDiagram>().point_event_right(idx);
                }
                Selection::None => {}
            }
        },
    );

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

    kb.bind(GLOBAL, 'C', "Clear diagram", |world| {
        let mode = world.get::<EditorState>().mode.clone();
        if mode == EditorMode::Normal {
            let diagram = world.get::<SequenceDiagram>();
            if !diagram.participants.is_empty() || !diagram.events.is_empty() {
                world.get_mut::<EditorState>().mode = EditorMode::ConfirmClear;
            }
        }
    });

    kb.bind(GLOBAL, 'E', "Export Mermaid", |world| {
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

    kb.bind(GLOBAL, KeyBinding::key(KeyCode::Enter), "Edit", |world| {
        let mode = world.get::<EditorState>().mode.clone();
        if mode != EditorMode::Normal {
            return;
        }

        let selection = world.get::<EditorState>().selection;
        match selection {
            Selection::Event(idx) => {
                let event_data = {
                    let diagram = world.get::<SequenceDiagram>();
                    diagram.events.get(idx).cloned()
                };
                match event_data {
                    Some(Event::Message { from, to, text }) => {
                        let editor = world.get_mut::<EditorState>();
                        editor.editing_event_index = Some(idx);
                        editor.message_from = Some(from);
                        editor.message_to = Some(to);
                        editor.input_buffer = text;
                        editor.selected_index = from;
                        editor.mode = EditorMode::EditSelectFrom;
                    }
                    Some(Event::Note {
                        position,
                        participant_start,
                        participant_end,
                        text,
                    }) => {
                        let editor = world.get_mut::<EditorState>();
                        editor.editing_event_index = Some(idx);
                        editor.note_position = position;
                        editor.note_participant_start = Some(participant_start);
                        editor.note_participant_end = Some(participant_end);
                        editor.input_buffer = text;
                        editor.selected_index = participant_start;
                        editor.mode = EditorMode::EditNoteParticipant;
                    }
                    None => {}
                }
            }
            Selection::Participant(idx) => {
                let name = {
                    let diagram = world.get::<SequenceDiagram>();
                    diagram.participants.get(idx).cloned()
                };
                if let Some(name) = name {
                    let editor = world.get_mut::<EditorState>();
                    editor.selected_index = idx;
                    editor.input_buffer = name;
                    editor.mode = EditorMode::RenameParticipant;
                }
            }
            Selection::None => {}
        }
    });

    kb.bind(GLOBAL, 'r', "Rename", |world| {
        let mode = world.get::<EditorState>().mode.clone();
        if mode != EditorMode::Normal {
            return;
        }

        let selection = world.get::<EditorState>().selection;
        match selection {
            Selection::Event(idx) => {
                let event_data = {
                    let diagram = world.get::<SequenceDiagram>();
                    diagram.events.get(idx).cloned()
                };
                match event_data {
                    Some(Event::Message { from, to, text }) => {
                        let editor = world.get_mut::<EditorState>();
                        editor.editing_event_index = Some(idx);
                        editor.message_from = Some(from);
                        editor.message_to = Some(to);
                        editor.input_buffer = text;
                        editor.mode = EditorMode::EditMessage;
                    }
                    Some(Event::Note {
                        position,
                        participant_start,
                        participant_end,
                        text,
                    }) => {
                        let editor = world.get_mut::<EditorState>();
                        editor.editing_event_index = Some(idx);
                        editor.note_position = position;
                        editor.note_participant_start = Some(participant_start);
                        editor.note_participant_end = Some(participant_end);
                        editor.input_buffer = text;
                        editor.mode = EditorMode::EditNoteText;
                    }
                    None => {}
                }
            }
            Selection::Participant(idx) => {
                let name = {
                    let diagram = world.get::<SequenceDiagram>();
                    diagram.participants.get(idx).cloned()
                };
                if let Some(name) = name {
                    let editor = world.get_mut::<EditorState>();
                    editor.selected_index = idx;
                    editor.input_buffer = name;
                    editor.mode = EditorMode::RenameParticipant;
                }
            }
            Selection::None => {}
        }
    });
}

fn input_mode_keybindings(world: &mut World) {
    let kb = world.get_mut::<Keybindings>();

    kb.bind(
        INPUT_MODE,
        KeyBinding::key(KeyCode::Enter),
        "Confirm",
        handle_input_confirm,
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
            if mode.is_text_input() {
                world.get_mut::<EditorState>().input_buffer.pop();
            }
        },
    );

    kb.bind(INPUT_MODE, KeyBinding::key(KeyCode::Up), "Up", |world| {
        let mode = world.get::<EditorState>().mode.clone();
        if mode.is_selecting_note_position() {
            let editor = world.get_mut::<EditorState>();
            editor.note_position = editor.note_position.prev();
        } else {
            handle_input_mode_nav(world, -1);
        }
    });

    kb.bind(
        INPUT_MODE,
        KeyBinding::key(KeyCode::Down),
        "Down",
        |world| {
            let mode = world.get::<EditorState>().mode.clone();
            if mode.is_selecting_note_position() {
                let editor = world.get_mut::<EditorState>();
                editor.note_position = editor.note_position.next();
            } else {
                handle_input_mode_nav(world, 1);
            }
        },
    );

    kb.bind(
        INPUT_MODE,
        KeyBinding::key(KeyCode::Left),
        "Left",
        |world| {
            let mode = world.get::<EditorState>().mode.clone();
            if mode.is_selecting_note_position() {
                let editor = world.get_mut::<EditorState>();
                editor.note_position = editor.note_position.prev();
            } else {
                handle_input_mode_nav(world, -1);
            }
        },
    );

    kb.bind(
        INPUT_MODE,
        KeyBinding::key(KeyCode::Right),
        "Right",
        |world| {
            let mode = world.get::<EditorState>().mode.clone();
            if mode.is_selecting_note_position() {
                let editor = world.get_mut::<EditorState>();
                editor.note_position = editor.note_position.next();
            } else {
                handle_input_mode_nav(world, 1);
            }
        },
    );

    kb.bind(INPUT_MODE, 'k', "Up", |world| {
        let mode = world.get::<EditorState>().mode.clone();
        if mode.is_selecting_participant() {
            handle_input_mode_nav(world, -1);
        } else if mode.is_selecting_note_position() {
            let editor = world.get_mut::<EditorState>();
            editor.note_position = editor.note_position.prev();
        } else if mode.is_text_input() {
            world.get_mut::<EditorState>().input_buffer.push('k');
        }
    });

    kb.bind(INPUT_MODE, 'j', "Down", |world| {
        let mode = world.get::<EditorState>().mode.clone();
        if mode.is_selecting_participant() {
            handle_input_mode_nav(world, 1);
        } else if mode.is_selecting_note_position() {
            let editor = world.get_mut::<EditorState>();
            editor.note_position = editor.note_position.next();
        } else if mode.is_text_input() {
            world.get_mut::<EditorState>().input_buffer.push('j');
        }
    });

    kb.bind(INPUT_MODE, 'h', "Left", |world| {
        let mode = world.get::<EditorState>().mode.clone();
        if mode.is_selecting_participant() {
            handle_input_mode_nav(world, -1);
        } else if mode.is_selecting_note_position() {
            let editor = world.get_mut::<EditorState>();
            editor.note_position = editor.note_position.prev();
        } else if mode.is_text_input() {
            world.get_mut::<EditorState>().input_buffer.push('h');
        }
    });

    kb.bind(INPUT_MODE, 'l', "Right", |world| {
        let mode = world.get::<EditorState>().mode.clone();
        if mode.is_selecting_participant() {
            handle_input_mode_nav(world, 1);
        } else if mode.is_selecting_note_position() {
            let editor = world.get_mut::<EditorState>();
            editor.note_position = editor.note_position.next();
        } else if mode.is_text_input() {
            world.get_mut::<EditorState>().input_buffer.push('l');
        }
    });

    for digit in '1'..='9' {
        kb.bind(INPUT_MODE, digit, "Select", move |world| {
            let mode = world.get::<EditorState>().mode.clone();
            let participant_count = world.get::<SequenceDiagram>().participant_count();
            let num = digit.to_digit(10).unwrap() as usize;

            if mode.is_selecting_participant() {
                if num >= 1 && num <= participant_count {
                    world.get_mut::<EditorState>().selected_index = num - 1;
                }
            } else if mode.is_text_input() {
                world.get_mut::<EditorState>().input_buffer.push(digit);
            }
        });
    }

    kb.bind_any(INPUT_MODE, |world, key| {
        let mode = world.get::<EditorState>().mode.clone();
        if mode.is_text_input()
            && let KeyCode::Char(c) = key.code
        {
            world.get_mut::<EditorState>().input_buffer.push(c);
        }
    });
}

fn confirm_mode_keybindings(world: &mut World) {
    let kb = world.get_mut::<Keybindings>();

    kb.bind(CONFIRM_MODE, 'y', "Yes", |world| {
        let diagram = world.get_mut::<SequenceDiagram>();
        diagram.participants.clear();
        diagram.events.clear();
        world.get_mut::<EditorState>().reset();
    });

    kb.bind_many(CONFIRM_MODE, keys!['n', KeyCode::Esc], "No", |world| {
        world.get_mut::<EditorState>().reset();
    });
}

#[allow(
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn handle_input_confirm(world: &mut World) {
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
                && let (Some(from), Some(to)) = (editor_state.message_from, editor_state.message_to)
            {
                let diagram = world.get_mut::<SequenceDiagram>();
                let event_idx = match editor_state.insert_after_index {
                    Some(usize::MAX) => {
                        diagram.events.insert(0, Event::Message { from, to, text });
                        0
                    }
                    Some(after_idx) => {
                        diagram.insert_message(after_idx, from, to, text);
                        after_idx + 1
                    }
                    None => {
                        diagram.add_message(from, to, text);
                        diagram.event_count() - 1
                    }
                };
                world.get_mut::<EditorState>().selection = Selection::Event(event_idx);
            }
            world.get_mut::<EditorState>().reset();
        }
        EditorMode::EditMessage => {
            save_event_changes(world);
        }
        EditorMode::EditSelectFrom => {
            let selected = world.get::<EditorState>().selected_index;
            let editor = world.get_mut::<EditorState>();
            editor.message_from = Some(selected);
            editor.mode = EditorMode::EditSelectTo;
            editor.selected_index = editor.message_to.unwrap_or(selected);
        }
        EditorMode::EditSelectTo => {
            let selected = world.get::<EditorState>().selected_index;
            let editor = world.get_mut::<EditorState>();
            editor.message_to = Some(selected);
            editor.mode = EditorMode::EditMessage;
        }
        EditorMode::RenameParticipant => {
            let editor_state = world.get::<EditorState>();
            let name = editor_state.input_buffer.trim().to_string();
            let idx = editor_state.selected_index;
            if !name.is_empty() {
                let diagram = world.get_mut::<SequenceDiagram>();
                if let Some(participant) = diagram.participants.get_mut(idx) {
                    *participant = name;
                }
            }
            world.get_mut::<EditorState>().reset();
        }
        EditorMode::SelectNoteParticipant => {
            let selected = world.get::<EditorState>().selected_index;
            let editor = world.get_mut::<EditorState>();
            editor.note_participant_start = Some(selected);
            editor.mode = EditorMode::SelectNotePosition;
        }
        EditorMode::SelectNotePosition => {
            let editor = world.get::<EditorState>();
            let position = editor.note_position;
            if position == NotePosition::Over {
                let editor = world.get_mut::<EditorState>();
                editor.mode = EditorMode::SelectNoteEndParticipant;
                editor.selected_index = editor.note_participant_start.unwrap_or(0);
            } else {
                let editor = world.get_mut::<EditorState>();
                editor.note_participant_end = editor.note_participant_start;
                editor.mode = EditorMode::InputNoteText;
                editor.input_buffer.clear();
            }
        }
        EditorMode::SelectNoteEndParticipant => {
            let selected = world.get::<EditorState>().selected_index;
            let editor = world.get_mut::<EditorState>();
            editor.note_participant_end = Some(selected);
            editor.mode = EditorMode::InputNoteText;
            editor.input_buffer.clear();
        }
        EditorMode::InputNoteText => {
            let editor_state = world.get::<EditorState>().clone();
            let text = editor_state.input_buffer.trim().to_string();
            if !text.is_empty()
                && let (Some(start), Some(end)) = (
                    editor_state.note_participant_start,
                    editor_state.note_participant_end,
                )
            {
                let diagram = world.get_mut::<SequenceDiagram>();
                let position = editor_state.note_position;
                let event_idx = match editor_state.insert_after_index {
                    Some(usize::MAX) => {
                        diagram.events.insert(
                            0,
                            Event::Note {
                                position,
                                participant_start: start,
                                participant_end: end,
                                text,
                            },
                        );
                        0
                    }
                    Some(after_idx) => {
                        diagram.insert_note(after_idx, position, start, end, text);
                        after_idx + 1
                    }
                    None => {
                        diagram.add_note(position, start, end, text);
                        diagram.event_count() - 1
                    }
                };
                world.get_mut::<EditorState>().selection = Selection::Event(event_idx);
            }
            world.get_mut::<EditorState>().reset();
        }
        EditorMode::EditNoteParticipant => {
            let selected = world.get::<EditorState>().selected_index;
            let editor = world.get_mut::<EditorState>();
            editor.note_participant_start = Some(selected);
            editor.mode = EditorMode::EditNotePosition;
        }
        EditorMode::EditNotePosition => {
            let editor = world.get::<EditorState>();
            let position = editor.note_position;
            if position == NotePosition::Over {
                let editor = world.get_mut::<EditorState>();
                editor.mode = EditorMode::EditNoteEndParticipant;
                editor.selected_index = editor.note_participant_end.unwrap_or(0);
            } else {
                let editor = world.get_mut::<EditorState>();
                editor.note_participant_end = editor.note_participant_start;
                editor.mode = EditorMode::EditNoteText;
            }
        }
        EditorMode::EditNoteEndParticipant => {
            let selected = world.get::<EditorState>().selected_index;
            let editor = world.get_mut::<EditorState>();
            editor.note_participant_end = Some(selected);
            editor.mode = EditorMode::EditNoteText;
        }
        EditorMode::EditNoteText => {
            save_note_changes(world);
        }
        _ => {}
    }
}

fn save_event_changes(world: &mut World) {
    let editor_state = world.get::<EditorState>().clone();
    let text = editor_state.input_buffer.trim().to_string();
    if let Some(idx) = editor_state.editing_event_index
        && let (Some(from), Some(to)) = (editor_state.message_from, editor_state.message_to)
        && !text.is_empty()
    {
        let diagram = world.get_mut::<SequenceDiagram>();
        if let Some(Event::Message {
            from: f,
            to: t,
            text: txt,
        }) = diagram.events.get_mut(idx)
        {
            *f = from;
            *t = to;
            *txt = text;
        }
    }
    world.get_mut::<EditorState>().reset();
}

fn save_note_changes(world: &mut World) {
    let editor_state = world.get::<EditorState>().clone();
    let text = editor_state.input_buffer.trim().to_string();
    if let Some(idx) = editor_state.editing_event_index
        && let (Some(start), Some(end)) = (
            editor_state.note_participant_start,
            editor_state.note_participant_end,
        )
        && !text.is_empty()
    {
        let diagram = world.get_mut::<SequenceDiagram>();
        if let Some(Event::Note {
            position: pos,
            participant_start: s,
            participant_end: e,
            text: txt,
        }) = diagram.events.get_mut(idx)
        {
            *pos = editor_state.note_position;
            *s = start;
            *e = end;
            *txt = text;
        }
    }
    world.get_mut::<EditorState>().reset();
}

#[allow(
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn handle_input_mode_nav(world: &mut World, delta: i32) {
    let mode = world.get::<EditorState>().mode.clone();
    if !mode.is_selecting_participant() {
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
        EditorMode::ConfirmClear => vec![CONFIRM_MODE],
        _ => vec![INPUT_MODE],
    }
}

pub fn render(frame: &mut Frame, world: &mut World) {
    let area = frame.area();
    world.get_mut::<AppState>().area = area;

    let [diagram_area, status_area] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(area);

    let is_empty = world.get::<SequenceDiagram>().participants.is_empty();

    if is_empty {
        render_empty_state(frame, diagram_area, world);
    } else {
        render_sequence(frame, diagram_area, world);
    }

    let editor = world.get::<EditorState>();
    let theme = world.get::<Theme>();
    let keybindings = world.get::<Keybindings>();

    render_status_bar(frame, status_area, world);

    match &editor.mode {
        EditorMode::InputParticipant
        | EditorMode::InputMessage
        | EditorMode::EditMessage
        | EditorMode::RenameParticipant
        | EditorMode::InputNoteText
        | EditorMode::EditNoteText => {
            render_input_popup(frame, world);
        }
        EditorMode::SelectFrom
        | EditorMode::SelectTo
        | EditorMode::EditSelectFrom
        | EditorMode::EditSelectTo => {
            render_dual_participant_selector(frame, area, world);
        }
        EditorMode::SelectNoteParticipant
        | EditorMode::SelectNoteEndParticipant
        | EditorMode::EditNoteParticipant
        | EditorMode::EditNoteEndParticipant => {
            render_note_participant_selector(frame, area, world);
        }
        EditorMode::SelectNotePosition | EditorMode::EditNotePosition => {
            render_note_position_selector(frame, area, world);
        }
        EditorMode::Help => {
            let active = vec![GLOBAL];
            render_help(frame, area, theme, keybindings, &active);
        }
        EditorMode::ConfirmClear => {
            render_confirm_dialog(frame, world);
        }
        EditorMode::Normal => {}
    }
}

#[allow(clippy::cast_possible_truncation)]
fn render_empty_state(frame: &mut Frame, area: Rect, world: &World) {
    let theme = world.get::<Theme>();
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
fn render_dual_participant_selector(frame: &mut Frame, area: Rect, world: &World) {
    let editor = world.get::<EditorState>();
    let diagram = world.get::<SequenceDiagram>();
    let theme = world.get::<Theme>();

    let participants = &diagram.participants;
    let from_idx = editor.message_from;
    let cursor = editor.selected_index;
    let selecting_from = editor.mode.is_selecting_from();
    let is_edit_mode = matches!(
        editor.mode,
        EditorMode::EditSelectFrom | EditorMode::EditSelectTo
    );
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

        let from_selected = from_idx == Some(i) && !is_edit_mode;
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

#[allow(clippy::cast_possible_truncation)]
fn render_note_participant_selector(frame: &mut Frame, area: Rect, world: &World) {
    let editor = world.get::<EditorState>();
    let diagram = world.get::<SequenceDiagram>();
    let theme = world.get::<Theme>();

    let participants = &diagram.participants;
    let cursor = editor.selected_index;
    let is_selecting_end = matches!(
        editor.mode,
        EditorMode::SelectNoteEndParticipant | EditorMode::EditNoteEndParticipant
    );

    let popup_width = 40.min(area.width.saturating_sub(4));
    let popup_height = (participants.len() as u16 + 4).min(area.height.saturating_sub(4));

    let popup_area = centered_rect(popup_width, popup_height, area);

    frame.render_widget(ratatui::widgets::Clear, popup_area);

    let title = if is_selecting_end {
        " Note: End Participant "
    } else {
        " Note: Participant "
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(theme.border);

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    for (i, name) in participants.iter().enumerate() {
        if i as u16 >= inner.height {
            break;
        }

        let y = inner.y + i as u16;
        let is_cursor = cursor == i;
        let prefix = if is_cursor { "▶ " } else { "  " };
        let style = if is_cursor {
            theme.selected
        } else {
            theme.text
        };

        frame.render_widget(
            Paragraph::new(format!("{prefix}{name}")).style(style),
            Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            },
        );
    }
}

#[allow(clippy::cast_possible_truncation)]
fn render_note_position_selector(frame: &mut Frame, area: Rect, world: &World) {
    let editor = world.get::<EditorState>();
    let diagram = world.get::<SequenceDiagram>();
    let theme = world.get::<Theme>();

    let current_position = editor.note_position;
    let participant_name = editor
        .note_participant_start
        .and_then(|i| diagram.participants.get(i))
        .map_or("?", String::as_str);

    let positions = [
        (NotePosition::Right, "Right of"),
        (NotePosition::Left, "Left of"),
        (NotePosition::Over, "Over"),
    ];

    let popup_width = 40.min(area.width.saturating_sub(4));
    let popup_height = 7;

    let popup_area = centered_rect(popup_width, popup_height, area);

    frame.render_widget(ratatui::widgets::Clear, popup_area);

    let block = Block::default()
        .title(format!(" Note Position ({participant_name}) "))
        .borders(Borders::ALL)
        .border_style(theme.border);

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    for (i, (pos, label)) in positions.iter().enumerate() {
        let y = inner.y + i as u16;
        let is_selected = *pos == current_position;
        let prefix = if is_selected { "▶ " } else { "  " };
        let style = if is_selected {
            theme.selected
        } else {
            theme.text
        };

        frame.render_widget(
            Paragraph::new(format!("{prefix}{label}")).style(style),
            Rect {
                x: inner.x,
                y,
                width: inner.width,
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
