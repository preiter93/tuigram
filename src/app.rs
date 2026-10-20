use crate::{
    core::{
        ArrowStyle, BoxColor, DEFAULT_BOX_COLOR, Event, FragmentKind, NotePosition, Participant,
        ParticipantBox, SequenceDiagram,
    },
    render::{box_color, render_sequence},
    theme::Theme,
    ui::{
        EditorMode, EditorState, Selection, confirm::render_confirm_dialog, help::render_help,
        input::render_input_popup, scroll::ScrollState, status_bar::render_status_bar,
    },
};
use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use std::{fs, mem, path::PathBuf};
use tui_world::keys;
use tui_world::{InputEvent, KeyBinding, Keybindings, WidgetId, World};

const DEFAULT_FILE: &str = "diagram.mmd";
const MAX_UNDO_HISTORY: usize = 100;

pub const NORMAL: WidgetId = WidgetId("Normal");
pub const HELP: WidgetId = WidgetId("Help");
pub const CONFIRM: WidgetId = WidgetId("Confirm");
pub const TEXT_INPUT: WidgetId = WidgetId("TextInput");
pub const SELECT_PARTICIPANT: WidgetId = WidgetId("SelectParticipant");
pub const SELECT_POSITION: WidgetId = WidgetId("SelectPosition");
pub const SELECT_BOX_COLOR: WidgetId = WidgetId("SelectBoxColor");
pub const SELECT_BOX_END: WidgetId = WidgetId("SelectBoxEnd");
pub const SELECT_FRAME_END: WidgetId = WidgetId("SelectFrameEnd");
pub const SELECT_FRAME_KIND: WidgetId = WidgetId("SelectFrameKind");

pub struct AppState {
    pub should_quit: bool,
    pub area: Rect,
    pub file_path: PathBuf,
    pub saved_mermaid: String,
    pub pending_clipboard: Option<String>,
    undo_stack: Vec<(SequenceDiagram, Selection)>,
    redo_stack: Vec<(SequenceDiagram, Selection)>,
    committed: SequenceDiagram,
}

impl AppState {
    fn is_dirty(&self, diagram: &SequenceDiagram) -> bool {
        diagram.to_mermaid() != self.saved_mermaid
    }

    fn file_name(&self) -> String {
        self.file_path
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
    }
}

pub fn setup_world(world: &mut World, diagram: SequenceDiagram, path: Option<PathBuf>) {
    world.insert(Theme::default());
    world.insert(AppState {
        should_quit: false,
        area: Rect::default(),
        file_path: path.unwrap_or_else(|| PathBuf::from(DEFAULT_FILE)),
        saved_mermaid: diagram.to_mermaid(),
        pending_clipboard: None,
        undo_stack: Vec::new(),
        redo_stack: Vec::new(),
        committed: diagram.clone(),
    });
    world.insert(diagram);
    world.insert(EditorState::new());
    world.insert(ScrollState::new());

    normal_keybindings(world);
    help_keybindings(world);
    select_participant_keybindings(world);
    select_position_keybindings(world);
    text_input_keybindings(world);
    confirm_keybindings(world);
    select_box_color_keybindings(world);
    select_box_end_keybindings(world);
    select_frame_end_keybindings(world);
    select_frame_kind_keybindings(world);
}

fn cycle_arrow_style(world: &mut World, forward: bool) {
    let selection = world.get::<EditorState>().selection;
    let Selection::Event(idx) = selection else {
        return;
    };

    let diagram = world.get_mut::<SequenceDiagram>();
    let style = if forward {
        diagram.next_arrow_style(idx)
    } else {
        diagram.prev_arrow_style(idx)
    };

    if let Some(style) = style {
        let token = style.as_mermaid_str();
        world
            .get_mut::<EditorState>()
            .set_status(format!("Arrow: {token}"));
    }
}

fn toggle_actor(world: &mut World) {
    let selection = world.get::<EditorState>().selection;
    let Selection::Participant(idx) = selection else {
        return;
    };

    let actor = world.get_mut::<SequenceDiagram>().toggle_actor(idx);

    if let Some(actor) = actor {
        let kind = if actor { "Actor" } else { "Participant" };
        world.get_mut::<EditorState>().set_status(kind);
    }
}

fn toggle_autonumber(world: &mut World) {
    let on = world.get_mut::<SequenceDiagram>().toggle_autonumber();
    let status = if on {
        "Autonumber on"
    } else {
        "Autonumber off"
    };
    world.get_mut::<EditorState>().set_status(status);
}

fn start_rename_participant(world: &mut World, idx: usize) {
    let name = world
        .get::<SequenceDiagram>()
        .participants
        .get(idx)
        .map(|p| p.display_name().to_string());

    if let Some(name) = name {
        let editor = world.get_mut::<EditorState>();
        editor.selected_index = idx;
        editor.input_buffer = name;
        editor.mode = EditorMode::RenameParticipant;
    }
}

fn request_quit(world: &mut World) {
    let diagram = world.get::<SequenceDiagram>();
    let app = world.get::<AppState>();

    if app.is_dirty(diagram) {
        world.get_mut::<EditorState>().mode = EditorMode::ConfirmQuit;
    } else {
        world.get_mut::<AppState>().should_quit = true;
    }
}

fn undo(world: &mut World) {
    let Some((previous, selection)) = world.get_mut::<AppState>().undo_stack.pop() else {
        world.get_mut::<EditorState>().set_status("Nothing to undo");
        return;
    };

    let current_selection = world.get::<EditorState>().selection;
    let app = world.get_mut::<AppState>();
    let current = mem::replace(&mut app.committed, previous.clone());
    app.redo_stack.push((current, current_selection));

    restore_diagram(world, previous, selection, "Undo");
}

fn redo(world: &mut World) {
    let Some((next, selection)) = world.get_mut::<AppState>().redo_stack.pop() else {
        world.get_mut::<EditorState>().set_status("Nothing to redo");
        return;
    };

    let current_selection = world.get::<EditorState>().selection;
    let app = world.get_mut::<AppState>();
    let current = mem::replace(&mut app.committed, next.clone());
    app.undo_stack.push((current, current_selection));

    restore_diagram(world, next, selection, "Redo");
}

fn restore_diagram(
    world: &mut World,
    diagram: SequenceDiagram,
    selection: Selection,
    status: &str,
) {
    let participant_count = diagram.participant_count();
    let event_count = diagram.event_count();
    let fragment_count = diagram.fragments.len();
    let box_count = diagram.boxes.len();
    let selection = match selection {
        Selection::Section(frame_idx, k)
            if diagram
                .fragments
                .get(frame_idx)
                .is_none_or(|f| k >= f.sections.len()) =>
        {
            Selection::Fragment(frame_idx)
        }
        other => other,
    };
    *world.get_mut::<SequenceDiagram>() = diagram;

    let editor = world.get_mut::<EditorState>();
    editor.selection = selection.clamp(participant_count, event_count, fragment_count, box_count);
    editor.last_participant_index = None;
    editor.set_status(status);

    let scroll = world.get_mut::<ScrollState>();
    scroll.offset = scroll.offset.min(event_count.saturating_sub(1));
}

fn save_to(world: &mut World, path: PathBuf) {
    let mermaid = world.get::<SequenceDiagram>().to_mermaid();

    if let Err(e) = fs::write(&path, &mermaid) {
        world
            .get_mut::<EditorState>()
            .set_error(format!("Save failed: {e}"));
        return;
    }

    let app = world.get_mut::<AppState>();
    app.file_path = path;
    app.saved_mermaid = mermaid;
    let message = format!("Saved to {}", app.file_name());
    world.get_mut::<EditorState>().set_status(message);
}

fn expand_tilde(input: &str) -> PathBuf {
    let Some(home) = std::env::var_os("HOME") else {
        return PathBuf::from(input);
    };

    if input == "~" {
        return PathBuf::from(home);
    }

    if let Some(rest) = input.strip_prefix("~/") {
        return PathBuf::from(home).join(rest);
    }

    PathBuf::from(input)
}

fn copy_diagram(world: &mut World) {
    let mermaid = world.get::<SequenceDiagram>().to_mermaid();
    let lines = mermaid.lines().count();

    world.get_mut::<AppState>().pending_clipboard = Some(mermaid);
    world
        .get_mut::<EditorState>()
        .set_status(format!("Copied {lines} lines"));
}

fn start_insert(world: &mut World, mode: EditorMode, before: bool) {
    let participant_count = world.get::<SequenceDiagram>().participant_count();
    let needed = if mode == EditorMode::SelectFrom { 2 } else { 1 };
    if participant_count < needed {
        return;
    }

    let diagram = world.get::<SequenceDiagram>();
    let (anchor, frame, before) = match world.get::<EditorState>().selection {
        Selection::Event(idx) => (Some(idx), None, before),
        Selection::Fragment(idx) => (None, Some(idx), before),
        // Below a divider is before its first event, above it after the last one of the section above.
        Selection::Section(idx, k) => {
            match diagram.fragments.get(idx).and_then(|f| f.sections.get(k)) {
                Some((at, _)) if before => (Some(at - 1), None, false),
                Some((at, _)) => (Some(*at), None, true),
                None => (None, None, before),
            }
        }
        _ => (None, None, before),
    };
    let editor = world.get_mut::<EditorState>();
    editor.mode = mode;
    editor.selected_index = 0;
    editor.message_from = None;
    editor.message_to = None;
    editor.note_position = NotePosition::Right;
    editor.note_participant_start = None;
    editor.note_participant_end = None;
    editor.insert_anchor = anchor;
    editor.insert_before = before;
    editor.insert_frame = frame;
}

/// Inserts at the remembered spot. With a frame selected, `insert_before`
/// puts the event above the frame instead of inside it.
fn insert_new_event(world: &mut World, editor: &EditorState, event: Event) {
    let diagram = world.get_mut::<SequenceDiagram>();
    let event_idx = match editor.insert_frame {
        Some(frame_idx) => diagram.insert_at_frame(frame_idx, !editor.insert_before, event),
        None => diagram.insert_relative(editor.insert_anchor, editor.insert_before, event),
    };
    world.get_mut::<EditorState>().selection = Selection::Event(event_idx);
}

/// Lets `k` return to the participant column the selection left from.
fn remember_participant_column(world: &mut World, selection: Selection) {
    let diagram = world.get::<SequenceDiagram>();
    let column = match selection {
        Selection::Participant(idx) => Some(idx),
        Selection::Box(idx) => diagram.boxes.get(idx).map(|b| b.start),
        _ => None,
    };

    if column.is_some() {
        world.get_mut::<EditorState>().last_participant_index = column;
    }
}

fn normal_keybindings(world: &mut World) {
    let kb = world.get_mut::<Keybindings>();

    kb.bind(NORMAL, KeyBinding::ctrl('c'), "Quit", request_quit);

    kb.bind(NORMAL, 'p', "Add participant", |world| {
        let editor = world.get_mut::<EditorState>();
        editor.mode = EditorMode::InputParticipant;
        editor.input_buffer.clear();
    });

    kb.bind(NORMAL, 'm', "Insert message after", |world| {
        start_insert(world, EditorMode::SelectFrom, false);
    });

    kb.bind(NORMAL, 'M', "Insert message before", |world| {
        start_insert(world, EditorMode::SelectFrom, true);
    });

    kb.bind(NORMAL, 'n', "Insert note after", |world| {
        start_insert(world, EditorMode::SelectNoteParticipant, false);
    });

    kb.bind(NORMAL, 'N', "Insert note before", |world| {
        start_insert(world, EditorMode::SelectNoteParticipant, true);
    });

    kb.bind(NORMAL, 'D', "Delete selected", |world| {
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
            Selection::Fragment(idx) => {
                remove_frame(world, idx);
            }
            Selection::Section(idx, k) => remove_section(world, idx, k),
            Selection::Box(idx) => remove_selected_box(world, idx),
            Selection::None => {
                let diagram = world.get_mut::<SequenceDiagram>();
                if let Some(last) = diagram.event_count().checked_sub(1) {
                    diagram.remove_event(last);
                }
            }
        }
    });

    kb.bind(NORMAL, 'B', "Remove box", |world| {
        let selection = world.get::<EditorState>().selection;
        match selection {
            Selection::Participant(idx) => {
                let box_idx = world.get::<SequenceDiagram>().box_index_of(idx);
                if let Some(box_idx) = box_idx {
                    world.get_mut::<SequenceDiagram>().remove_box_at(box_idx);
                } else {
                    world.get_mut::<EditorState>().set_status("No box here");
                }
            }
            Selection::Box(idx) => remove_selected_box(world, idx),
            _ => {}
        }
    });

    kb.bind(NORMAL, 'f', "Add/edit frame", start_frame);

    kb.bind(NORMAL, 's', "Add section divider", start_section);

    kb.bind(NORMAL, 'F', "Remove frame", |world| {
        let selection = world.get::<EditorState>().selection;
        let diagram = world.get::<SequenceDiagram>();
        if let Some(frame_idx) = selected_frame(diagram, selection) {
            remove_frame(world, frame_idx);
        } else {
            world.get_mut::<EditorState>().set_status("No frame here");
        }
    });

    kb.bind_many(
        NORMAL,
        keys!['l', KeyCode::Right],
        "Select Right",
        |world| {
            let selection = world.get::<EditorState>().selection;
            let next = selection.right(world.get::<SequenceDiagram>());
            world.get_mut::<EditorState>().selection = next;
        },
    );

    kb.bind_many(NORMAL, keys!['h', KeyCode::Left], "Select Left", |world| {
        let selection = world.get::<EditorState>().selection;
        let next = selection.left(world.get::<SequenceDiagram>());
        world.get_mut::<EditorState>().selection = next;
    });

    kb.bind_many(NORMAL, keys!['j', KeyCode::Down], "Select Down", |world| {
        let selection = world.get::<EditorState>().selection;
        let next = selection.down(world.get::<SequenceDiagram>());

        remember_participant_column(world, selection);
        world.get_mut::<EditorState>().selection = next;
    });

    kb.bind(NORMAL, 'g', "First event", |world| {
        let selection = world.get::<EditorState>().selection;
        let next = selection.first_event(world.get::<SequenceDiagram>());

        remember_participant_column(world, selection);
        world.get_mut::<EditorState>().selection = next;
    });

    kb.bind(NORMAL, 'G', "Last event", |world| {
        let selection = world.get::<EditorState>().selection;
        let next = selection.last_event(world.get::<SequenceDiagram>());

        remember_participant_column(world, selection);
        world.get_mut::<EditorState>().selection = next;
    });

    kb.bind_many(NORMAL, keys!['k', KeyCode::Up], "Select Up", |world| {
        let diagram = world.get::<SequenceDiagram>();
        let participant_count = diagram.participant_count();
        let editor = world.get::<EditorState>();
        let selection = editor.selection;
        let last_participant = editor.last_participant_index;
        let mut next = selection.up(diagram);

        let leaves_diagram = matches!(
            selection,
            Selection::Event(_) | Selection::Fragment(_) | Selection::Section(..)
        );
        if leaves_diagram
            && matches!(next, Selection::Participant(_))
            && let Some(idx) = last_participant
            && idx < participant_count
        {
            next = Selection::Participant(idx);
        }

        world.get_mut::<EditorState>().selection = next;
    });

    kb.bind_many(
        NORMAL,
        keys!['H', KeyBinding::new(KeyCode::Left, KeyModifiers::SHIFT)],
        "Move participant/box left",
        |world| {
            let selection = world.get::<EditorState>().selection;
            match selection {
                Selection::Participant(idx) if idx > 0 => {
                    world
                        .get_mut::<SequenceDiagram>()
                        .swap_participants(idx, idx - 1);
                    world.get_mut::<EditorState>().selection = Selection::Participant(idx - 1);
                }
                Selection::Event(idx) => world.get_mut::<SequenceDiagram>().point_event_left(idx),
                Selection::Box(idx) => move_selected_box(world, idx, false),
                _ => {}
            }
        },
    );

    kb.bind_many(
        NORMAL,
        keys!['L', KeyBinding::new(KeyCode::Right, KeyModifiers::SHIFT)],
        "Move participant/box right",
        |world| {
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
                Selection::Event(idx) => world.get_mut::<SequenceDiagram>().point_event_right(idx),
                Selection::Box(idx) => move_selected_box(world, idx, true),
                Selection::Fragment(_) | Selection::Section(..) | Selection::None => {}
            }
        },
    );

    kb.bind(NORMAL, 'a', "Next arrow style", |world| {
        cycle_arrow_style(world, true);
    });

    kb.bind(NORMAL, 'A', "Previous arrow style", |world| {
        cycle_arrow_style(world, false);
    });

    kb.bind(NORMAL, 't', "Toggle actor", toggle_actor);

    // Some terminals report symbol keys with SHIFT set.
    kb.bind_many(
        NORMAL,
        keys!['#', KeyBinding::shift('#')],
        "Toggle autonumber",
        toggle_autonumber,
    );

    kb.bind_many(
        NORMAL,
        keys!['J', KeyBinding::new(KeyCode::Down, KeyModifiers::SHIFT)],
        "Move event/frame down",
        |world| move_selection(world, true),
    );

    kb.bind_many(
        NORMAL,
        keys!['K', KeyBinding::new(KeyCode::Up, KeyModifiers::SHIFT)],
        "Move event/frame up",
        |world| move_selection(world, false),
    );

    kb.bind_many(
        NORMAL,
        keys!['e', KeyBinding::key(KeyCode::Enter)],
        "Edit",
        |world| {
            let selection = world.get::<EditorState>().selection;
            match selection {
                Selection::Event(idx) => {
                    let event_data = {
                        let diagram = world.get::<SequenceDiagram>();
                        diagram.events.get(idx).cloned()
                    };
                    match event_data {
                        Some(Event::Message { from, to, text, .. }) => {
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
                    start_rename_participant(world, idx);
                }
                Selection::Fragment(_) => start_frame(world),
                Selection::Section(idx, k) => start_section_edit(world, idx, k),
                Selection::Box(idx) => start_box_edit(world, idx),
                Selection::None => {}
            }
        },
    );

    kb.bind(NORMAL, 'r', "Rename", |world| {
        let selection = world.get::<EditorState>().selection;
        match selection {
            Selection::Event(idx) => {
                let event_data = {
                    let diagram = world.get::<SequenceDiagram>();
                    diagram.events.get(idx).cloned()
                };
                match event_data {
                    Some(Event::Message { from, to, text, .. }) => {
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
                start_rename_participant(world, idx);
            }
            Selection::Fragment(_) => start_frame(world),
            Selection::Section(idx, k) => start_section_edit(world, idx, k),
            Selection::Box(idx) => start_box_edit(world, idx),
            Selection::None => {}
        }
    });

    kb.bind(NORMAL, 'b', "Add box", start_box);

    kb.bind(NORMAL, 'C', "Clear diagram", |world| {
        let diagram = world.get::<SequenceDiagram>();
        if !diagram.participants.is_empty() || !diagram.events.is_empty() {
            world.get_mut::<EditorState>().mode = EditorMode::ConfirmClear;
        }
    });

    kb.bind(NORMAL, 'S', "Save", |world| {
        let path = world.get::<AppState>().file_path.display().to_string();
        let editor = world.get_mut::<EditorState>();
        editor.mode = EditorMode::InputSavePath;
        editor.input_buffer = path;
    });

    kb.bind(NORMAL, 'y', "Copy to clipboard", copy_diagram);

    kb.bind(NORMAL, 'u', "Undo", undo);
    kb.bind_many(NORMAL, keys!['U', KeyBinding::ctrl('r')], "Redo", redo);

    kb.bind_many(
        NORMAL,
        keys!['?', KeyBinding::shift('?')],
        "Help",
        |world| {
            world.get_mut::<EditorState>().mode = EditorMode::Help;
        },
    );

    kb.bind(
        NORMAL,
        KeyBinding::key(KeyCode::Esc),
        "Clear selection",
        |world| {
            world.get_mut::<EditorState>().clear_selection();
        },
    );
}

fn help_keybindings(world: &mut World) {
    let kb = world.get_mut::<Keybindings>();

    kb.bind_any(HELP, |world, _key| {
        world.get_mut::<EditorState>().mode = EditorMode::Normal;
    });
}

fn select_participant_keybindings(world: &mut World) {
    let kb = world.get_mut::<Keybindings>();

    kb.bind(
        SELECT_PARTICIPANT,
        KeyBinding::key(KeyCode::Enter),
        "Confirm",
        handle_input_confirm,
    );

    kb.bind(
        SELECT_PARTICIPANT,
        KeyBinding::key(KeyCode::Esc),
        "Cancel",
        |world| {
            world.get_mut::<EditorState>().reset();
        },
    );

    kb.bind_many(
        SELECT_PARTICIPANT,
        keys!['h', 'k', KeyCode::Left, KeyCode::Up],
        "Previous",
        |world| {
            handle_participant_nav(world, -1);
        },
    );

    kb.bind_many(
        SELECT_PARTICIPANT,
        keys!['j', 'l', KeyCode::Right, KeyCode::Down],
        "Next",
        |world| {
            handle_participant_nav(world, 1);
        },
    );

    for digit in '1'..='9' {
        kb.bind(SELECT_PARTICIPANT, digit, "Select", move |world| {
            let participant_count = world.get::<SequenceDiagram>().participant_count();
            let num = digit.to_digit(10).unwrap() as usize;
            if num >= 1 && num <= participant_count {
                world.get_mut::<EditorState>().selected_index = num - 1;
            }
        });
    }
}

fn select_position_keybindings(world: &mut World) {
    let kb = world.get_mut::<Keybindings>();

    kb.bind(
        SELECT_POSITION,
        KeyBinding::key(KeyCode::Enter),
        "Confirm",
        handle_input_confirm,
    );

    kb.bind(
        SELECT_POSITION,
        KeyBinding::key(KeyCode::Esc),
        "Cancel",
        |world| {
            world.get_mut::<EditorState>().reset();
        },
    );

    kb.bind_many(
        SELECT_POSITION,
        keys!['h', 'k', KeyCode::Left, KeyCode::Up],
        "Previous",
        |world| {
            let editor = world.get_mut::<EditorState>();
            editor.note_position = editor.note_position.prev();
        },
    );

    kb.bind_many(
        SELECT_POSITION,
        keys!['j', 'l', KeyCode::Right, KeyCode::Down],
        "Next",
        |world| {
            let editor = world.get_mut::<EditorState>();
            editor.note_position = editor.note_position.next();
        },
    );
}

fn text_input_keybindings(world: &mut World) {
    let kb = world.get_mut::<Keybindings>();

    kb.bind(
        TEXT_INPUT,
        KeyBinding::key(KeyCode::Enter),
        "Confirm",
        handle_input_confirm,
    );

    kb.bind(
        TEXT_INPUT,
        KeyBinding::new(KeyCode::Enter, KeyModifiers::ALT),
        "Line break",
        |world| {
            let editor = world.get_mut::<EditorState>();
            if editor.mode.is_message_text_input() {
                editor.input_buffer.push_str("<br/>");
            }
        },
    );

    kb.bind(
        TEXT_INPUT,
        KeyBinding::key(KeyCode::Esc),
        "Cancel",
        |world| {
            world.get_mut::<EditorState>().cancel();
        },
    );

    kb.bind(
        TEXT_INPUT,
        KeyBinding::key(KeyCode::Backspace),
        "Delete",
        |world| {
            world.get_mut::<EditorState>().input_buffer.pop();
        },
    );

    kb.bind_any(TEXT_INPUT, |world, key| {
        // AltGr arrives as Ctrl+Alt on Windows, so only plain Ctrl is a shortcut.
        let is_ctrl_shortcut = key.modifiers.contains(KeyModifiers::CONTROL)
            && !key.modifiers.contains(KeyModifiers::ALT);

        if let KeyCode::Char(c) = key.code
            && !is_ctrl_shortcut
        {
            world.get_mut::<EditorState>().input_buffer.push(c);
        }
    });
}

fn confirm_keybindings(world: &mut World) {
    let kb = world.get_mut::<Keybindings>();

    kb.bind_many(CONFIRM, keys!['y', KeyCode::Enter], "Yes", |world| {
        let mode = world.get::<EditorState>().mode.clone();

        match mode {
            EditorMode::ConfirmQuit => world.get_mut::<AppState>().should_quit = true,
            EditorMode::ConfirmClear => {
                let diagram = world.get_mut::<SequenceDiagram>();
                diagram.participants.clear();
                diagram.events.clear();
                diagram.boxes.clear();
                diagram.fragments.clear();
                world.get_mut::<EditorState>().reset();
            }
            _ => {}
        }
    });

    kb.bind_many(CONFIRM, keys!['n', KeyCode::Esc], "No", |world| {
        world.get_mut::<EditorState>().reset();
    });
}

fn start_box(world: &mut World) {
    let selection = world.get::<EditorState>().selection;
    let diagram = world.get::<SequenceDiagram>();
    if diagram.participant_count() == 0 {
        world
            .get_mut::<EditorState>()
            .set_error("Add a participant first (p)");
        return;
    }

    let existing_box = match selection {
        Selection::Box(idx) => Some(idx),
        Selection::Participant(idx) => diagram.box_index_of(idx),
        _ => None,
    };
    if let Some(idx) = existing_box {
        world.get_mut::<EditorState>().selection = Selection::Box(idx);
        start_box_edit(world, idx);
        return;
    }

    let free = |i: &usize| diagram.box_index_of(*i).is_none();
    let start = match selection {
        Selection::Participant(idx) => Some(idx),
        _ => (0..diagram.participant_count()).find(free),
    };
    let editor = world.get_mut::<EditorState>();
    let Some(start) = start else {
        editor.set_error("No free participants to box");
        return;
    };

    editor.selection_before = selection;
    editor.selection = Selection::Participant(start);
    editor.box_start = Some(start);
    editor.box_end = None;
    editor.mode = EditorMode::SelectBoxEnd;
}

fn start_frame(world: &mut World) {
    let selection = world.get::<EditorState>().selection;
    let diagram = world.get::<SequenceDiagram>();
    let existing = selected_frame(diagram, selection).map(|i| diagram.fragments[i].clone());
    let first_free = (0..diagram.event_count()).find(|&i| diagram.fragment_at(i).is_none());
    let has_events = diagram.event_count() > 0;
    let editor = world.get_mut::<EditorState>();

    if let Some(fragment) = existing {
        editor.frame_kind = fragment.kind;
        editor.input_buffer = fragment.label;
        editor.mode = EditorMode::SelectFrameKind;
        return;
    }

    let start = match selection {
        Selection::Event(idx) => Some(idx),
        _ => first_free,
    };
    let Some(start) = start else {
        let message = if has_events {
            "No free messages to frame"
        } else {
            "Add a message first (m)"
        };
        editor.set_error(message);
        return;
    };

    editor.selection_before = selection;
    editor.selection = Selection::Event(start);
    editor.frame_start = Some(start);
    editor.mode = EditorMode::SelectFrameEnd;
}

fn selected_frame(diagram: &SequenceDiagram, selection: Selection) -> Option<usize> {
    match selection {
        Selection::Event(idx) => diagram.fragment_at(idx),
        Selection::Fragment(idx) | Selection::Section(idx, _) => Some(idx),
        _ => None,
    }
}

fn remove_frame(world: &mut World, frame_idx: usize) {
    let diagram = world.get_mut::<SequenceDiagram>();
    let Some(frame) = diagram.fragments.get(frame_idx) else {
        return;
    };
    let start = frame.start;
    diagram.remove_fragment_at(frame_idx);

    let editor = world.get_mut::<EditorState>();
    if matches!(
        editor.selection,
        Selection::Fragment(_) | Selection::Section(..)
    ) {
        editor.selection = Selection::Event(start);
    }
    editor.set_status("Frame removed");
}

fn start_section(world: &mut World) {
    let selection = world.get::<EditorState>().selection;
    let diagram = world.get::<SequenceDiagram>();
    let target = section_target(diagram, selection);
    let editor = world.get_mut::<EditorState>();

    match target {
        Ok(at) => {
            editor.selection_before = selection;
            editor.section_start = Some(at);
            editor.input_buffer.clear();
            editor.mode = EditorMode::InputSectionLabel;
        }
        Err(message) => editor.set_error(message),
    }
}

/// The event a new divider would go above, or why there is none.
fn section_target(diagram: &SequenceDiagram, selection: Selection) -> Result<usize, String> {
    let idx = match selection {
        Selection::Event(idx) => idx,
        Selection::Section(frame_idx, k) => {
            let section = diagram
                .fragments
                .get(frame_idx)
                .and_then(|f| f.sections.get(k));
            section.map_or(usize::MAX, |(at, _)| *at)
        }
        _ => return Err("Select an event inside alt, par or critical".to_string()),
    };
    let Some(frame_idx) = diagram.fragment_at(idx) else {
        return Err("Select an event inside alt, par or critical".to_string());
    };

    let frame = &diagram.fragments[frame_idx];
    if frame.kind.divider().is_none() {
        return Err(format!("'{}' has no sections", frame.kind.as_mermaid_str()));
    }
    if idx == frame.start {
        return Err("Pick an event below the frame's first one".to_string());
    }
    if diagram.section_at(idx).is_some() {
        return Err("A section already starts here".to_string());
    }
    Ok(idx)
}

fn start_section_edit(world: &mut World, frame_idx: usize, k: usize) {
    let label = world
        .get::<SequenceDiagram>()
        .fragments
        .get(frame_idx)
        .and_then(|f| f.sections.get(k))
        .map(|(_, label)| label.clone());

    if let Some(label) = label {
        let editor = world.get_mut::<EditorState>();
        editor.input_buffer = label;
        editor.mode = EditorMode::InputSectionLabel;
    }
}

fn remove_section(world: &mut World, frame_idx: usize, k: usize) {
    let diagram = world.get_mut::<SequenceDiagram>();
    let Some(frame) = diagram.fragments.get(frame_idx) else {
        return;
    };
    let first_of_merged = if k == 0 {
        frame.start
    } else {
        frame.sections[k - 1].0
    };
    diagram.remove_section(frame_idx, k);

    let editor = world.get_mut::<EditorState>();
    editor.selection = Selection::Event(first_of_merged);
    editor.set_status("Section removed");
}

fn move_selection(world: &mut World, down: bool) {
    match world.get::<EditorState>().selection {
        Selection::Event(idx) => move_selected_event(world, idx, down),
        Selection::Fragment(idx) => {
            let new_idx = world.get_mut::<SequenceDiagram>().move_fragment(idx, down);
            world.get_mut::<EditorState>().selection = Selection::Fragment(new_idx);
        }
        Selection::Section(idx, k) => {
            world
                .get_mut::<SequenceDiagram>()
                .move_section(idx, k, down);
        }
        _ => {}
    }
}

fn move_selected_box(world: &mut World, idx: usize, right: bool) {
    let new_idx = world.get_mut::<SequenceDiagram>().move_box(idx, right);
    world.get_mut::<EditorState>().selection = Selection::Box(new_idx);
}

fn remove_selected_box(world: &mut World, idx: usize) {
    let diagram = world.get_mut::<SequenceDiagram>();
    let Some(start) = diagram.boxes.get(idx).map(|b| b.start) else {
        return;
    };
    diagram.remove_box_at(idx);

    let editor = world.get_mut::<EditorState>();
    editor.selection = Selection::Participant(start);
    editor.set_status("Box removed");
}

/// A color written into the label (`rgb(...)`) can't be edited with the picker.
fn label_holds_color(b: &ParticipantBox) -> bool {
    b.color.is_none() && (b.label.starts_with("rgb(") || b.label.starts_with("rgba("))
}

fn start_box_edit(world: &mut World, idx: usize) {
    let Some(b) = world.get::<SequenceDiagram>().boxes.get(idx).cloned() else {
        return;
    };

    let editor = world.get_mut::<EditorState>();
    editor.box_start = None;
    editor.box_end = None;
    editor.box_color = b.color.unwrap_or(DEFAULT_BOX_COLOR);
    editor.input_buffer.clone_from(&b.label);
    editor.mode = if label_holds_color(&b) {
        EditorMode::InputBoxLabel
    } else {
        EditorMode::SelectBoxColor
    };
}

fn move_selected_event(world: &mut World, idx: usize, down: bool) {
    let diagram = world.get_mut::<SequenceDiagram>();
    let frames_before = diagram.fragments.len();
    let sections_before = section_count(diagram);
    let new_idx = diagram.move_event(idx, down);
    let frame_removed = diagram.fragments.len() < frames_before;
    let section_removed = section_count(diagram) < sections_before;

    let editor = world.get_mut::<EditorState>();
    editor.selection = Selection::Event(new_idx);
    if frame_removed {
        editor.set_status("Frame removed");
    } else if section_removed {
        editor.set_status("Section removed");
    }
}

fn section_count(diagram: &SequenceDiagram) -> usize {
    diagram.fragments.iter().map(|f| f.sections.len()).sum()
}

fn select_box_end_keybindings(world: &mut World) {
    let kb = world.get_mut::<Keybindings>();

    kb.bind(
        SELECT_BOX_END,
        KeyBinding::key(KeyCode::Enter),
        "Confirm",
        handle_input_confirm,
    );

    kb.bind(
        SELECT_BOX_END,
        KeyBinding::key(KeyCode::Esc),
        "Cancel",
        |world| {
            world.get_mut::<EditorState>().cancel();
        },
    );

    kb.bind_many(
        SELECT_BOX_END,
        keys!['h', KeyCode::Left],
        "Previous",
        |world| {
            let editor = world.get_mut::<EditorState>();
            if let Selection::Participant(idx) = editor.selection {
                editor.selection = Selection::Participant(idx.saturating_sub(1));
            }
        },
    );

    kb.bind_many(
        SELECT_BOX_END,
        keys!['l', KeyCode::Right],
        "Next",
        |world| {
            let count = world.get::<SequenceDiagram>().participant_count();
            let editor = world.get_mut::<EditorState>();
            if let Selection::Participant(idx) = editor.selection
                && idx + 1 < count
            {
                editor.selection = Selection::Participant(idx + 1);
            }
        },
    );
}

fn select_frame_end_keybindings(world: &mut World) {
    let kb = world.get_mut::<Keybindings>();

    kb.bind(
        SELECT_FRAME_END,
        KeyBinding::key(KeyCode::Enter),
        "Confirm",
        handle_input_confirm,
    );

    kb.bind(
        SELECT_FRAME_END,
        KeyBinding::key(KeyCode::Esc),
        "Cancel",
        |world| {
            world.get_mut::<EditorState>().cancel();
        },
    );

    kb.bind_many(
        SELECT_FRAME_END,
        keys!['k', 'h', KeyCode::Up, KeyCode::Left],
        "Previous",
        |world| {
            let editor = world.get_mut::<EditorState>();
            if let Selection::Event(idx) = editor.selection {
                editor.selection = Selection::Event(idx.saturating_sub(1));
            }
        },
    );

    kb.bind_many(
        SELECT_FRAME_END,
        keys!['j', 'l', KeyCode::Down, KeyCode::Right],
        "Next",
        |world| {
            let event_count = world.get::<SequenceDiagram>().event_count();
            let editor = world.get_mut::<EditorState>();
            if let Selection::Event(idx) = editor.selection
                && idx + 1 < event_count
            {
                editor.selection = Selection::Event(idx + 1);
            }
        },
    );
}

fn select_frame_kind_keybindings(world: &mut World) {
    let kb = world.get_mut::<Keybindings>();

    kb.bind(
        SELECT_FRAME_KIND,
        KeyBinding::key(KeyCode::Enter),
        "Confirm",
        handle_input_confirm,
    );

    kb.bind(
        SELECT_FRAME_KIND,
        KeyBinding::key(KeyCode::Esc),
        "Cancel",
        |world| {
            world.get_mut::<EditorState>().cancel();
        },
    );

    kb.bind_many(
        SELECT_FRAME_KIND,
        keys!['h', 'k', KeyCode::Left, KeyCode::Up],
        "Previous",
        |world| {
            let editor = world.get_mut::<EditorState>();
            editor.frame_kind = editor.frame_kind.prev();
        },
    );

    kb.bind_many(
        SELECT_FRAME_KIND,
        keys!['j', 'l', KeyCode::Right, KeyCode::Down],
        "Next",
        |world| {
            let editor = world.get_mut::<EditorState>();
            editor.frame_kind = editor.frame_kind.next();
        },
    );
}

fn select_box_color_keybindings(world: &mut World) {
    let kb = world.get_mut::<Keybindings>();

    kb.bind(
        SELECT_BOX_COLOR,
        KeyBinding::key(KeyCode::Enter),
        "Confirm",
        handle_input_confirm,
    );

    kb.bind(
        SELECT_BOX_COLOR,
        KeyBinding::key(KeyCode::Esc),
        "Cancel",
        |world| {
            world.get_mut::<EditorState>().cancel();
        },
    );

    kb.bind_many(
        SELECT_BOX_COLOR,
        keys!['h', 'k', KeyCode::Left, KeyCode::Up],
        "Previous",
        |world| {
            let editor = world.get_mut::<EditorState>();
            editor.box_color = editor.box_color.prev();
        },
    );

    kb.bind_many(
        SELECT_BOX_COLOR,
        keys!['j', 'l', KeyCode::Right, KeyCode::Down],
        "Next",
        |world| {
            let editor = world.get_mut::<EditorState>();
            editor.box_color = editor.box_color.next();
        },
    );
}

fn handle_input_confirm(world: &mut World) {
    let mode = world.get::<EditorState>().mode.clone();
    match mode {
        EditorMode::InputParticipant => {
            let name = world.get::<EditorState>().input_buffer.trim().to_string();
            let selection = world.get::<EditorState>().selection;
            if !name.is_empty() {
                let diagram = world.get_mut::<SequenceDiagram>();
                match selection {
                    Selection::Box(idx) => diagram.add_participant_to_box(idx, &name),
                    _ => diagram.add_participant(&name),
                }
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
                let event = Event::Message {
                    from,
                    to,
                    text,
                    style: ArrowStyle::default(),
                };
                insert_new_event(world, &editor_state, event);
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
                world
                    .get_mut::<SequenceDiagram>()
                    .rename_participant(idx, &name);
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
                let event = Event::Note {
                    position: editor_state.note_position,
                    participant_start: start,
                    participant_end: end,
                    text,
                };
                insert_new_event(world, &editor_state, event);
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
        EditorMode::SelectBoxEnd => confirm_box_end(world),
        EditorMode::SelectBoxColor => {
            let editor = world.get_mut::<EditorState>();
            editor.mode = EditorMode::InputBoxLabel;
            if editor.box_start.is_some() {
                editor.input_buffer.clear();
            }
        }
        EditorMode::InputBoxLabel => {
            let editor_state = world.get::<EditorState>().clone();
            let label = editor_state.input_buffer.trim().to_string();
            world.get_mut::<EditorState>().reset();

            if let Selection::Box(idx) = editor_state.selection
                && editor_state.box_start.is_none()
            {
                let diagram = world.get_mut::<SequenceDiagram>();
                if let Some(b) = diagram.boxes.get_mut(idx) {
                    let kept_default =
                        b.color.is_none() && editor_state.box_color == DEFAULT_BOX_COLOR;
                    if !label_holds_color(b) && !kept_default {
                        b.color = Some(editor_state.box_color);
                    }
                    b.label = label;
                }
                return;
            }

            if let (Some(mut start), Some(mut end)) = (editor_state.box_start, editor_state.box_end)
            {
                if start > end {
                    std::mem::swap(&mut start, &mut end);
                }
                let ok = world.get_mut::<SequenceDiagram>().add_box(
                    label,
                    editor_state.box_color,
                    start,
                    end,
                );
                if !ok {
                    world
                        .get_mut::<EditorState>()
                        .set_error("Boxes cannot overlap");
                }
            }
        }
        EditorMode::SelectFrameEnd => confirm_frame_end(world),
        EditorMode::SelectFrameKind => {
            let editor = world.get_mut::<EditorState>();
            editor.mode = EditorMode::InputFrameLabel;
        }
        EditorMode::InputFrameLabel => confirm_frame_label(world),
        EditorMode::InputSectionLabel => confirm_section_label(world),
        EditorMode::InputSavePath => {
            let input = world.get::<EditorState>().input_buffer.trim().to_string();
            world.get_mut::<EditorState>().reset();
            if !input.is_empty() {
                save_to(world, expand_tilde(&input));
            }
        }
        _ => {}
    }
}

fn confirm_box_end(world: &mut World) {
    let editor = world.get::<EditorState>();
    let (Some(start), Selection::Participant(cursor)) = (editor.box_start, editor.selection) else {
        world.get_mut::<EditorState>().reset();
        return;
    };
    let (first, last) = (start.min(cursor), start.max(cursor));

    let overlaps = world.get::<SequenceDiagram>().box_overlaps(first, last);
    let editor = world.get_mut::<EditorState>();
    if overlaps {
        return;
    }

    editor.selection = Selection::Participant(first);
    editor.box_start = Some(first);
    editor.box_end = Some(last);
    editor.box_color = BoxColor::default();
    editor.mode = EditorMode::SelectBoxColor;
}

fn confirm_frame_end(world: &mut World) {
    let editor = world.get::<EditorState>();
    let (Some(start), Selection::Event(cursor)) = (editor.frame_start, editor.selection) else {
        world.get_mut::<EditorState>().reset();
        return;
    };
    let (first, last) = (start.min(cursor), start.max(cursor));

    let overlaps = world
        .get::<SequenceDiagram>()
        .fragment_overlaps(first, last);
    let editor = world.get_mut::<EditorState>();
    if overlaps {
        return;
    }

    editor.selection = Selection::Event(first);
    editor.frame_start = Some(first);
    editor.frame_end = Some(last);
    editor.frame_kind = FragmentKind::default();
    editor.input_buffer.clear();
    editor.mode = EditorMode::SelectFrameKind;
}

fn confirm_frame_label(world: &mut World) {
    let editor_state = world.get::<EditorState>().clone();
    let label = editor_state.input_buffer.trim().to_string();
    let kind = editor_state.frame_kind;
    world.get_mut::<EditorState>().reset();

    let diagram = world.get_mut::<SequenceDiagram>();
    let (Some(start), Some(end)) = (editor_state.frame_start, editor_state.frame_end) else {
        if let Some(frame_idx) = selected_frame(diagram, editor_state.selection) {
            change_frame(world, frame_idx, kind, label);
        }
        return;
    };

    if !diagram.add_fragment(kind, label, start, end) {
        world
            .get_mut::<EditorState>()
            .set_error("Frames cannot overlap");
    }
}

/// Kinds without dividers can't keep their sections.
fn change_frame(world: &mut World, frame_idx: usize, kind: FragmentKind, label: String) {
    let frame = &mut world.get_mut::<SequenceDiagram>().fragments[frame_idx];
    frame.kind = kind;
    frame.label = label;

    if kind.divider().is_some() || frame.sections.is_empty() {
        return;
    }
    frame.sections.clear();

    let editor = world.get_mut::<EditorState>();
    if matches!(editor.selection, Selection::Section(..)) {
        editor.selection = Selection::Fragment(frame_idx);
    }
    editor.set_status("Sections removed");
}

fn confirm_section_label(world: &mut World) {
    let editor_state = world.get::<EditorState>().clone();
    let label = editor_state.input_buffer.trim().to_string();
    world.get_mut::<EditorState>().reset();

    let diagram = world.get_mut::<SequenceDiagram>();
    let Some(at) = editor_state.section_start else {
        if let Selection::Section(frame_idx, k) = editor_state.selection
            && let Some(section) = diagram
                .fragments
                .get_mut(frame_idx)
                .and_then(|f| f.sections.get_mut(k))
        {
            section.1 = label;
        }
        return;
    };

    let Some(frame_idx) = diagram.fragment_at(at) else {
        return;
    };
    if let Some(k) = diagram.add_section(frame_idx, at, label) {
        world.get_mut::<EditorState>().selection = Selection::Section(frame_idx, k);
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
            ..
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

fn handle_participant_nav(world: &mut World, delta: i32) {
    let participant_count = world.get::<SequenceDiagram>().participant_count();
    if participant_count == 0 {
        return;
    }

    let editor = world.get_mut::<EditorState>();
    let current = editor.selected_index as i32;
    let new_idx = (current + delta).rem_euclid(participant_count as i32) as usize;
    editor.selected_index = new_idx;
}

pub fn handle_key(world: &mut World, key: KeyEvent) {
    let active = active_widgets(world);
    let selection = world.get::<EditorState>().selection;
    InputEvent::Key(key).handle(world, &active);
    record_history(world, selection);
}

/// `selection_before` is the cursor before the key, restored by undo.
fn record_history(world: &mut World, selection_before: Selection) {
    let diagram = world.get::<SequenceDiagram>();
    let app = world.get::<AppState>();
    if *diagram == app.committed {
        return;
    }

    let diagram = diagram.clone();
    let app = world.get_mut::<AppState>();
    let previous = mem::replace(&mut app.committed, diagram);
    app.undo_stack.push((previous, selection_before));
    app.redo_stack.clear();

    if app.undo_stack.len() > MAX_UNDO_HISTORY {
        app.undo_stack.remove(0);
    }
}

pub fn active_widgets(world: &World) -> Vec<WidgetId> {
    let mode = &world.get::<EditorState>().mode;
    match mode {
        EditorMode::Normal => vec![NORMAL],
        EditorMode::Help => vec![HELP],
        EditorMode::ConfirmClear | EditorMode::ConfirmQuit => vec![CONFIRM],
        EditorMode::SelectNotePosition | EditorMode::EditNotePosition => vec![SELECT_POSITION],
        EditorMode::SelectBoxColor => vec![SELECT_BOX_COLOR],
        EditorMode::SelectFrameEnd => vec![SELECT_FRAME_END],
        EditorMode::SelectFrameKind => vec![SELECT_FRAME_KIND],
        EditorMode::SelectBoxEnd => vec![SELECT_BOX_END],
        m if m.is_selecting_participant() => vec![SELECT_PARTICIPANT],
        m if m.is_text_input() => vec![TEXT_INPUT],
        _ => vec![],
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

    let app = world.get::<AppState>();
    let dirty = app.is_dirty(world.get::<SequenceDiagram>());
    render_status_bar(frame, status_area, world, &app.file_name(), dirty);

    match &editor.mode {
        EditorMode::InputParticipant
        | EditorMode::InputMessage
        | EditorMode::EditMessage
        | EditorMode::RenameParticipant
        | EditorMode::InputNoteText
        | EditorMode::EditNoteText
        | EditorMode::InputBoxLabel
        | EditorMode::InputFrameLabel
        | EditorMode::InputSectionLabel
        | EditorMode::InputSavePath => {
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
            render_help(frame, area, theme, keybindings, &[HELP]);
        }
        EditorMode::ConfirmClear | EditorMode::ConfirmQuit => {
            render_confirm_dialog(frame, world);
        }
        EditorMode::SelectBoxColor => {
            render_box_color_selector(frame, area, world);
        }
        EditorMode::SelectFrameKind => {
            render_frame_kind_selector(frame, area, world);
        }
        EditorMode::Normal | EditorMode::SelectBoxEnd | EditorMode::SelectFrameEnd => {}
    }
}

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

    for (i, participant) in participants.iter().enumerate() {
        let name = participant.display_name();
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

    for (i, participant) in participants.iter().enumerate() {
        let name = participant.display_name();
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

fn render_note_position_selector(frame: &mut Frame, area: Rect, world: &World) {
    let editor = world.get::<EditorState>();
    let diagram = world.get::<SequenceDiagram>();
    let theme = world.get::<Theme>();

    let current_position = editor.note_position;
    let participant_name = editor
        .note_participant_start
        .and_then(|i| diagram.participants.get(i))
        .map_or("?", Participant::display_name);

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

fn render_box_color_selector(frame: &mut Frame, area: Rect, world: &World) {
    use ratatui::style::Style;
    let editor = world.get::<EditorState>();
    let theme = world.get::<Theme>();

    let current_color = editor.box_color;
    let colors = BoxColor::all();

    let popup_width = 30.min(area.width.saturating_sub(4));
    let popup_height = (colors.len() as u16 + 4).min(area.height.saturating_sub(4));

    let popup_area = centered_rect(popup_width, popup_height, area);

    frame.render_widget(ratatui::widgets::Clear, popup_area);

    let block = Block::default()
        .title(" Box Color ")
        .borders(Borders::ALL)
        .border_style(theme.border);

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    for (i, color) in colors.iter().enumerate() {
        let y = inner.y + i as u16;
        if y >= inner.y + inner.height {
            break;
        }

        let is_selected = *color == current_color;
        let swatch_color = box_color(*color);
        let prefix = if is_selected { "\u{25b6} " } else { "  " };
        let name_style = if is_selected {
            theme.selected
        } else {
            theme.text
        };

        let line = Line::from(vec![
            Span::raw(prefix),
            Span::styled("\u{25a0} ", Style::default().fg(swatch_color)),
            Span::styled(color.as_mermaid_str(), name_style),
        ]);

        frame.render_widget(
            Paragraph::new(line),
            Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            },
        );
    }

    if inner.height > colors.len() as u16 {
        let hint_y = inner.y + inner.height - 1;
        frame.render_widget(
            Paragraph::new("Enter: confirm | Esc: cancel")
                .style(theme.muted)
                .alignment(Alignment::Right),
            Rect {
                x: inner.x,
                y: hint_y,
                width: inner.width,
                height: 1,
            },
        );
    }
}

fn render_frame_kind_selector(frame: &mut Frame, area: Rect, world: &World) {
    let editor = world.get::<EditorState>();
    let theme = world.get::<Theme>();

    let popup_width = 30.min(area.width.saturating_sub(4));
    let popup_height = (FragmentKind::ALL.len() as u16 + 2).min(area.height.saturating_sub(4));
    let popup_area = centered_rect(popup_width, popup_height, area);

    frame.render_widget(ratatui::widgets::Clear, popup_area);

    let block = Block::default()
        .title(" Frame Kind ")
        .borders(Borders::ALL)
        .border_style(theme.border);
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    for (i, kind) in FragmentKind::ALL.iter().enumerate() {
        let y = inner.y + i as u16;
        if y >= inner.y + inner.height {
            break;
        }

        let is_selected = *kind == editor.frame_kind;
        let prefix = if is_selected { "\u{25b6} " } else { "  " };
        let style = if is_selected {
            theme.selected
        } else {
            theme.text
        };

        frame.render_widget(
            Paragraph::new(format!("{prefix}{}", kind.as_mermaid_str())).style(style),
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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend, crossterm::event::KeyEvent};
    use std::path::Path;

    fn world_with(diagram: SequenceDiagram, path: Option<PathBuf>) -> World {
        let mut world = World::default();
        setup_world(&mut world, diagram, path);
        world
    }

    fn is_dirty(world: &World) -> bool {
        world
            .get::<AppState>()
            .is_dirty(world.get::<SequenceDiagram>())
    }

    #[test]
    fn loaded_diagram_with_comments_is_not_dirty() {
        let source =
            "sequenceDiagram\n%% comment\n\n  participant A\n  participant B\n\n  A->>B: hi\n";
        let diagram = SequenceDiagram::from_mermaid(source).unwrap();
        let world = world_with(diagram, None);

        assert!(!is_dirty(&world));
    }

    #[test]
    fn edit_makes_dirty_and_baseline_makes_clean() {
        let mut world = world_with(SequenceDiagram::new(), None);

        world.get_mut::<SequenceDiagram>().add_participant("A");
        assert!(is_dirty(&world));

        let mermaid = world.get::<SequenceDiagram>().to_mermaid();
        world.get_mut::<AppState>().saved_mermaid = mermaid;
        assert!(!is_dirty(&world));
    }

    #[test]
    fn default_path_is_diagram_mmd() {
        let world = world_with(SequenceDiagram::new(), None);

        assert_eq!(world.get::<AppState>().file_path, Path::new("diagram.mmd"));
        assert_eq!(world.get::<AppState>().file_name(), "diagram.mmd");
    }

    #[test]
    fn clean_quit_exits_immediately() {
        let mut world = world_with(SequenceDiagram::new(), None);

        request_quit(&mut world);

        assert!(world.get::<AppState>().should_quit);
    }

    #[test]
    fn dirty_quit_asks_and_yes_quits() {
        let mut world = world_with(SequenceDiagram::new(), None);
        world.get_mut::<SequenceDiagram>().add_participant("A");

        request_quit(&mut world);

        assert!(!world.get::<AppState>().should_quit);
        assert_eq!(world.get::<EditorState>().mode, EditorMode::ConfirmQuit);

        let active = active_widgets(&world);
        InputEvent::Key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE))
            .handle(&mut world, &active);

        assert!(world.get::<AppState>().should_quit);
    }

    #[test]
    fn dirty_quit_no_cancels() {
        let mut world = world_with(SequenceDiagram::new(), None);
        world.get_mut::<SequenceDiagram>().add_participant("A");
        request_quit(&mut world);

        let active = active_widgets(&world);
        InputEvent::Key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE))
            .handle(&mut world, &active);

        assert!(!world.get::<AppState>().should_quit);
        assert_eq!(world.get::<EditorState>().mode, EditorMode::Normal);
    }

    #[test]
    fn save_writes_file_and_clears_dirty() {
        let path = std::env::temp_dir().join(format!("tuigram_save_{}.mmd", std::process::id()));
        let mut world = world_with(SequenceDiagram::new(), Some(path.clone()));
        world.get_mut::<SequenceDiagram>().add_participant("A");
        assert!(is_dirty(&world));

        press_char(&mut world, 'S');
        press(&mut world, KeyCode::Enter);

        let written = fs::read_to_string(&path);
        let _ = fs::remove_file(&path);

        assert_eq!(
            written.unwrap(),
            world.get::<SequenceDiagram>().to_mermaid()
        );
        assert!(!is_dirty(&world));
    }

    fn press(world: &mut World, code: KeyCode) {
        handle_key(world, KeyEvent::new(code, KeyModifiers::NONE));
    }

    fn press_char(world: &mut World, c: char) {
        let modifiers = if c.is_uppercase() {
            KeyModifiers::SHIFT
        } else {
            KeyModifiers::NONE
        };
        handle_key(world, KeyEvent::new(KeyCode::Char(c), modifiers));
    }

    fn sample_world() -> World {
        let diagram = SequenceDiagram::from_mermaid(
            "sequenceDiagram\n participant A\n participant B\n A->>B: hi\n",
        )
        .unwrap();
        world_with(diagram, None)
    }

    fn status(world: &World) -> String {
        world
            .get::<EditorState>()
            .get_status()
            .map_or_else(String::new, |s| s.text.clone())
    }

    #[test]
    fn undo_restores_deleted_event_then_nothing_to_undo() {
        let mut world = sample_world();
        world.get_mut::<EditorState>().selection = Selection::Event(0);

        press_char(&mut world, 'D');
        assert_eq!(world.get::<SequenceDiagram>().event_count(), 0);

        press_char(&mut world, 'u');
        assert_eq!(world.get::<SequenceDiagram>().event_count(), 1);

        press_char(&mut world, 'u');
        assert_eq!(status(&world), "Nothing to undo");
    }

    #[test]
    fn typing_participant_is_one_undo_step() {
        let mut world = sample_world();

        press_char(&mut world, 'p');
        press_char(&mut world, 'X');
        press_char(&mut world, 'Y');
        press(&mut world, KeyCode::Enter);
        assert_eq!(world.get::<SequenceDiagram>().participant_count(), 3);

        press_char(&mut world, 'u');
        assert_eq!(world.get::<SequenceDiagram>().participant_count(), 2);
    }

    #[test]
    fn t_toggles_actor_and_undo_restores() {
        let mut world = sample_world();
        world.get_mut::<EditorState>().selection = Selection::Participant(0);

        press_char(&mut world, 't');
        assert!(world.get::<SequenceDiagram>().participants[0].actor);
        assert_eq!(status(&world), "Actor");
        assert!(is_dirty(&world));

        press_char(&mut world, 'u');
        assert!(!world.get::<SequenceDiagram>().participants[0].actor);
    }

    #[test]
    fn t_on_event_does_nothing() {
        let mut world = sample_world();
        world.get_mut::<EditorState>().selection = Selection::Event(0);

        press_char(&mut world, 't');
        assert!(
            world
                .get::<SequenceDiagram>()
                .participants
                .iter()
                .all(|p| !p.actor)
        );
        assert!(!is_dirty(&world));
    }

    #[test]
    fn redo_with_shift_u_and_ctrl_r() {
        let mut world = sample_world();
        world.get_mut::<EditorState>().selection = Selection::Event(0);
        press_char(&mut world, 'D');

        press_char(&mut world, 'u');
        press_char(&mut world, 'U');
        assert_eq!(world.get::<SequenceDiagram>().event_count(), 0);

        press_char(&mut world, 'u');
        handle_key(
            &mut world,
            KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
        );
        assert_eq!(world.get::<SequenceDiagram>().event_count(), 0);
        assert_eq!(status(&world), "Redo");

        press_char(&mut world, 'U');
        assert_eq!(status(&world), "Nothing to redo");
    }

    #[test]
    fn new_change_after_undo_clears_redo() {
        let mut world = sample_world();
        world.get_mut::<EditorState>().selection = Selection::Event(0);
        press_char(&mut world, 'D');
        press_char(&mut world, 'u');

        world.get_mut::<EditorState>().selection = Selection::Participant(1);
        press_char(&mut world, 'D');
        press_char(&mut world, 'U');

        assert_eq!(status(&world), "Nothing to redo");
    }

    #[test]
    fn undo_history_is_capped() {
        let mut world = world_with(SequenceDiagram::new(), None);

        for _ in 0..105 {
            press_char(&mut world, 'p');
            press_char(&mut world, 'A');
            press(&mut world, KeyCode::Enter);
        }

        assert_eq!(world.get::<AppState>().undo_stack.len(), MAX_UNDO_HISTORY);
    }

    #[test]
    fn undo_restores_selection_from_before_change() {
        let mut world = world_with(SequenceDiagram::new(), None);
        press_char(&mut world, 'p');
        press_char(&mut world, 'A');
        press(&mut world, KeyCode::Enter);
        world.get_mut::<EditorState>().selection = Selection::Participant(0);

        press_char(&mut world, 'p');
        press_char(&mut world, 'B');
        press(&mut world, KeyCode::Enter);
        world.get_mut::<EditorState>().selection = Selection::Participant(1);

        press_char(&mut world, 'u');

        assert_eq!(
            world.get::<EditorState>().selection,
            Selection::Participant(0)
        );
    }

    #[test]
    fn undo_cleans_dirty_marker() {
        let mut world = sample_world();
        world.get_mut::<EditorState>().selection = Selection::Event(0);

        press_char(&mut world, 'D');
        assert!(is_dirty(&world));

        press_char(&mut world, 'u');
        assert!(!is_dirty(&world));
    }

    #[test]
    fn undo_restores_cleared_diagram() {
        let mut world = sample_world();

        press_char(&mut world, 'C');
        press_char(&mut world, 'y');
        assert_eq!(world.get::<SequenceDiagram>().participant_count(), 0);

        press_char(&mut world, 'u');
        assert_eq!(world.get::<SequenceDiagram>().participant_count(), 2);
        assert_eq!(world.get::<SequenceDiagram>().event_count(), 1);
    }

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("tuigram-test-{}-{name}", std::process::id()))
    }

    fn type_text(world: &mut World, text: &str) {
        for c in text.chars() {
            press_char(world, c);
        }
    }

    #[test]
    fn y_sets_pending_clipboard_and_status() {
        let mut world = sample_world();

        press_char(&mut world, 'y');

        let expected = world.get::<SequenceDiagram>().to_mermaid();
        let lines = expected.lines().count();
        assert_eq!(
            world.get::<AppState>().pending_clipboard.as_deref(),
            Some(expected.as_str())
        );
        assert_eq!(status(&world), format!("Copied {lines} lines"));
    }

    #[test]
    fn capital_s_opens_prompt_prefilled_with_path() {
        let mut world = world_with(SequenceDiagram::new(), Some(PathBuf::from("a/b.mmd")));

        press_char(&mut world, 'S');

        let editor = world.get::<EditorState>();
        assert_eq!(editor.mode, EditorMode::InputSavePath);
        assert_eq!(editor.input_buffer, "a/b.mmd");
    }

    #[test]
    fn save_as_writes_updates_path_and_later_save_uses_it() {
        let path = temp_path("save-as.mmd");
        let mut world = sample_world();

        press_char(&mut world, 'S');
        for _ in 0..world.get::<EditorState>().input_buffer.len() {
            press(&mut world, KeyCode::Backspace);
        }
        type_text(&mut world, path.to_str().unwrap());
        press(&mut world, KeyCode::Enter);

        let written = fs::read_to_string(&path);
        let mermaid = world.get::<SequenceDiagram>().to_mermaid();
        assert_eq!(world.get::<AppState>().file_path, path);
        assert!(!is_dirty(&world));
        assert_eq!(world.get::<EditorState>().mode, EditorMode::Normal);

        world.get_mut::<SequenceDiagram>().add_participant("C");
        press_char(&mut world, 'S');
        press(&mut world, KeyCode::Enter);
        let saved = fs::read_to_string(&path);
        let _ = fs::remove_file(&path);

        assert_eq!(written.unwrap(), mermaid);
        assert!(saved.unwrap().contains("participant C"));
    }

    #[test]
    fn save_as_to_missing_dir_is_error_and_keeps_path() {
        let path = temp_path("missing-dir").join("nested").join("x.mmd");
        let mut world = world_with(SequenceDiagram::new(), Some(PathBuf::from("keep.mmd")));
        world.get_mut::<EditorState>().mode = EditorMode::InputSavePath;
        world.get_mut::<EditorState>().input_buffer = path.display().to_string();

        press(&mut world, KeyCode::Enter);

        let status = world.get::<EditorState>().get_status().unwrap();
        assert!(status.is_error);
        assert!(status.text.starts_with("Save failed"));
        assert_eq!(world.get::<AppState>().file_path, Path::new("keep.mmd"));
        assert!(!path.exists());
    }

    #[test]
    fn save_as_escape_cancels() {
        let mut world = sample_world();

        press_char(&mut world, 'S');
        press(&mut world, KeyCode::Esc);

        assert_eq!(world.get::<EditorState>().mode, EditorMode::Normal);
        assert_eq!(world.get::<AppState>().file_path, Path::new("diagram.mmd"));
    }

    #[test]
    fn expand_tilde_cases() {
        let home = std::env::var_os("HOME").map(PathBuf::from);

        assert_eq!(expand_tilde("a/b.mmd"), Path::new("a/b.mmd"));
        assert_eq!(expand_tilde("/abs/b.mmd"), Path::new("/abs/b.mmd"));
        assert_eq!(expand_tilde("~other/x"), Path::new("~other/x"));

        if let Some(home) = home {
            assert_eq!(expand_tilde("~"), home);
            assert_eq!(expand_tilde("~/x.mmd"), home.join("x.mmd"));
        }
    }

    #[test]
    fn help_closes_on_any_key_without_side_effects() {
        let mut world = sample_world();

        press_char(&mut world, '?');
        assert_eq!(world.get::<EditorState>().mode, EditorMode::Help);
        press_char(&mut world, 'p');
        assert_eq!(world.get::<EditorState>().mode, EditorMode::Normal);

        world.get_mut::<EditorState>().selection = Selection::Event(0);
        press_char(&mut world, '?');
        press_char(&mut world, 'D');
        assert_eq!(world.get::<EditorState>().mode, EditorMode::Normal);
        assert_eq!(world.get::<SequenceDiagram>().event_count(), 1);

        press_char(&mut world, '?');
        press_char(&mut world, '?');
        assert_eq!(world.get::<EditorState>().mode, EditorMode::Normal);

        press_char(&mut world, '?');
        handle_key(
            &mut world,
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        );
        assert_eq!(world.get::<EditorState>().mode, EditorMode::Normal);
        assert!(!world.get::<AppState>().should_quit);
    }

    #[test]
    fn overlapping_box_stays_in_end_mode() {
        let mut world = sample_world_with_box();
        world.get_mut::<EditorState>().selection = Selection::Participant(2);

        press_char(&mut world, 'b');
        press_char(&mut world, 'h');
        press(&mut world, KeyCode::Enter);

        assert!(world.get::<EditorState>().get_status().is_none());
        assert_eq!(world.get::<EditorState>().mode, EditorMode::SelectBoxEnd);
        assert_eq!(world.get::<EditorState>().box_start, Some(2));
        assert_eq!(selection(&world), Selection::Participant(1));

        press_char(&mut world, 'l');
        press(&mut world, KeyCode::Enter);
        assert_eq!(world.get::<EditorState>().mode, EditorMode::SelectBoxColor);
    }

    fn framed_world() -> World {
        let diagram = SequenceDiagram::from_mermaid(
            "sequenceDiagram\n participant A\n participant B\n A->>B: one\n A->>B: two\n A->>B: three\n",
        )
        .unwrap();
        world_with(diagram, None)
    }

    fn frames(world: &World) -> Vec<(FragmentKind, String, usize, usize)> {
        world
            .get::<SequenceDiagram>()
            .fragments
            .iter()
            .map(|f| (f.kind, f.label.clone(), f.start, f.end))
            .collect()
    }

    fn add_frame_via_keys(world: &mut World, label: &str) {
        press_char(world, 'f');
        press_char(world, 'j');
        press(world, KeyCode::Enter);
        press_char(world, 'j');
        press_char(world, 'j');
        press(world, KeyCode::Enter);
        type_text(world, label);
        press(world, KeyCode::Enter);
    }

    #[test]
    fn f_creates_frame_and_undo_removes_it() {
        let mut world = framed_world();
        world.get_mut::<EditorState>().selection = Selection::Event(0);

        add_frame_via_keys(&mut world, "Retry");

        assert_eq!(
            frames(&world),
            vec![(FragmentKind::Opt, "Retry".to_string(), 0, 1)]
        );
        assert_eq!(world.get::<EditorState>().mode, EditorMode::Normal);
        assert_eq!(world.get::<EditorState>().selection, Selection::Event(0));

        press_char(&mut world, 'u');
        assert_eq!(frames(&world).len(), 0);
    }

    #[test]
    fn f_on_framed_event_edits_kind_and_label() {
        let mut world = framed_world();
        world.get_mut::<EditorState>().selection = Selection::Event(0);
        add_frame_via_keys(&mut world, "Retry");

        press_char(&mut world, 'f');
        assert_eq!(world.get::<EditorState>().frame_kind, FragmentKind::Opt);
        press_char(&mut world, 'k');
        press(&mut world, KeyCode::Enter);
        assert_eq!(world.get::<EditorState>().input_buffer, "Retry");
        press(&mut world, KeyCode::Backspace);
        press_char(&mut world, 'X');
        press(&mut world, KeyCode::Enter);

        assert_eq!(
            frames(&world),
            vec![(FragmentKind::Alt, "RetrX".to_string(), 0, 1)]
        );
    }

    #[test]
    fn capital_f_removes_frame_or_reports_none() {
        let mut world = framed_world();
        world.get_mut::<EditorState>().selection = Selection::Event(2);
        press_char(&mut world, 'F');
        assert_eq!(status(&world), "No frame here");

        world.get_mut::<EditorState>().selection = Selection::Event(0);
        add_frame_via_keys(&mut world, "");
        press_char(&mut world, 'F');
        assert_eq!(frames(&world).len(), 0);
    }

    #[test]
    fn f_over_existing_frame_is_an_error() {
        let mut world = framed_world();
        world
            .get_mut::<SequenceDiagram>()
            .add_fragment(FragmentKind::Loop, String::new(), 1, 1);
        world.get_mut::<EditorState>().selection = Selection::Event(0);

        press_char(&mut world, 'f');
        press_char(&mut world, 'j');
        press(&mut world, KeyCode::Enter);

        assert!(world.get::<EditorState>().get_status().is_none());
        assert_eq!(world.get::<EditorState>().mode, EditorMode::SelectFrameEnd);
        assert_eq!(world.get::<EditorState>().frame_start, Some(0));
        assert_eq!(selection(&world), Selection::Event(1));
        assert_eq!(frames(&world).len(), 1);

        press_char(&mut world, 'k');
        press(&mut world, KeyCode::Enter);
        assert_eq!(world.get::<EditorState>().mode, EditorMode::SelectFrameKind);
    }

    #[test]
    fn f_escape_cancels_and_restores_selection() {
        let mut world = framed_world();
        world.get_mut::<EditorState>().selection = Selection::Event(0);

        press_char(&mut world, 'f');
        press_char(&mut world, 'j');
        press(&mut world, KeyCode::Esc);

        assert_eq!(world.get::<EditorState>().mode, EditorMode::Normal);
        assert_eq!(world.get::<EditorState>().selection, Selection::Event(0));
        assert_eq!(frames(&world).len(), 0);
    }

    #[test]
    fn shift_j_on_last_framed_event_leaves_frame() {
        let mut world = framed_world();
        world
            .get_mut::<SequenceDiagram>()
            .add_fragment(FragmentKind::Loop, String::new(), 0, 1);
        world.get_mut::<EditorState>().selection = Selection::Event(1);

        press_char(&mut world, 'J');

        assert_eq!(
            frames(&world),
            vec![(FragmentKind::Loop, String::new(), 0, 0)]
        );
        assert_eq!(world.get::<EditorState>().selection, Selection::Event(1));

        world.get_mut::<EditorState>().selection = Selection::Event(0);
        press_char(&mut world, 'K');
        assert_eq!(status(&world), "Frame removed");
        assert_eq!(frames(&world).len(), 0);
    }

    #[test]
    fn frame_is_drawn_with_header_and_footer() {
        let mut world = framed_world();
        world.get_mut::<SequenceDiagram>().add_fragment(
            FragmentKind::Loop,
            "Every minute".to_string(),
            0,
            1,
        );

        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|frame| render(frame, &mut world)).unwrap();

        let buffer = terminal.backend().buffer();
        let screen: String = buffer
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(screen.contains("┌ loop [Every minute]"));
        assert!(screen.contains('└'));
    }

    #[test]
    fn long_frame_label_stays_inside_narrow_window() {
        let diagram = SequenceDiagram::from_mermaid(
            "sequenceDiagram\n participant A\n participant B\n A->>B: one\n",
        )
        .unwrap();
        let mut world = world_with(diagram, None);
        world.get_mut::<SequenceDiagram>().add_fragment(
            FragmentKind::Loop,
            "A very long label that cannot possibly fit".to_string(),
            0,
            0,
        );

        let mut terminal = Terminal::new(TestBackend::new(30, 15)).unwrap();
        terminal.draw(|frame| render(frame, &mut world)).unwrap();

        let buffer = terminal.backend().buffer();
        for y in 1..13 {
            assert_eq!(buffer[(0, y)].symbol(), "│", "left border row {y}");
            assert_eq!(buffer[(29, y)].symbol(), "│", "right border row {y}");
        }
        let header: String = (0..30).map(|x| buffer[(x, 6)].symbol()).collect();
        assert!(header.contains("┌ loop"), "{header}");
        assert_eq!(buffer[(28, 6)].symbol(), "┐", "{header}");
    }

    #[test]
    fn right_of_note_renders_on_narrow_terminals() {
        let source = "sequenceDiagram\nparticipant A\nparticipant B\nparticipant C\nNote right of C: hello there\n";
        for width in [3, 8, 20] {
            let diagram = SequenceDiagram::from_mermaid(source).unwrap();
            let mut world = world_with(diagram, None);
            let mut terminal = Terminal::new(TestBackend::new(width, 14)).unwrap();
            terminal.draw(|frame| render(frame, &mut world)).unwrap();
        }
    }

    fn frame_screen(source: &str, width: u16) -> Vec<String> {
        let diagram = SequenceDiagram::from_mermaid(source).unwrap();
        let mut world = world_with(diagram, None);
        let mut terminal = Terminal::new(TestBackend::new(width, 16)).unwrap();
        terminal.draw(|frame| render(frame, &mut world)).unwrap();

        let buffer = terminal.backend().buffer();
        (0..16)
            .map(|y| (0..width).map(|x| buffer[(x, y)].symbol()).collect())
            .collect()
    }

    fn col_of(row: &str, needle: &str) -> usize {
        let byte = row.find(needle).unwrap();
        row[..byte].chars().count()
    }

    #[test]
    fn self_message_keeps_lifeline_on_middle_row() {
        let screen = frame_screen("sequenceDiagram\nparticipant A\nA->>A: self\n", 40);

        let row = screen.iter().position(|r| r.contains('◀')).unwrap();
        let lifeline_col = col_of(&screen[row], "◀");
        assert_eq!(screen[row - 1].chars().nth(lifeline_col), Some('│'));
    }

    #[test]
    fn frame_encloses_note_over_single_participant() {
        let screen = frame_screen(
            "sequenceDiagram\nparticipant A\nparticipant B\nopt\nNote over A: optional\nend\n",
            60,
        );

        let note = screen.iter().find(|r| r.contains("optional")).unwrap();
        let header = screen.iter().find(|r| r.contains("┌ opt")).unwrap();
        let frame_left = col_of(header, "┌");
        let frame_right = col_of(header, "┐");
        let text_left = col_of(note, "optional");

        assert!(frame_left < text_left - 1);
        assert!(text_left + "optional".len() + 1 < frame_right);
        assert_eq!(note.chars().nth(frame_left), Some('│'));
        assert_eq!(note.chars().nth(frame_right), Some('│'));
    }

    fn rows_containing(screen: &[String], needle: &str) -> Vec<usize> {
        screen
            .iter()
            .enumerate()
            .filter(|(_, r)| r.contains(needle))
            .map(|(i, _)| i)
            .collect()
    }

    #[test]
    fn alt_enter_does_nothing_in_label_and_path_prompts() {
        let mut world = sample_world();
        press_char(&mut world, 'p');
        type_text(&mut world, "a");
        handle_key(&mut world, KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT));
        assert_eq!(world.get::<EditorState>().input_buffer, "a");

        let mut world = world_with(SequenceDiagram::new(), Some(PathBuf::from("a.mmd")));
        press_char(&mut world, 'S');
        handle_key(&mut world, KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT));
        assert_eq!(world.get::<EditorState>().input_buffer, "a.mmd");
    }

    #[test]
    fn two_line_message_draws_arrow_below_both_lines() {
        let screen = frame_screen(
            "sequenceDiagram\nparticipant A\nparticipant B\nA->>B: first<br/>second\n",
            40,
        );

        let first = rows_containing(&screen, "first")[0];
        let second = rows_containing(&screen, "second")[0];
        let arrow = rows_containing(&screen, "▶")[0];

        assert_eq!(second, first + 1);
        assert_eq!(arrow, second + 1);
    }

    #[test]
    fn two_line_right_note_is_one_reversed_block() {
        let diagram = SequenceDiagram::from_mermaid(
            "sequenceDiagram\nparticipant A\nNote right of A: top<br>longer line\n",
        )
        .unwrap();
        let mut world = world_with(diagram, None);
        let mut terminal = Terminal::new(TestBackend::new(40, 16)).unwrap();
        terminal.draw(|frame| render(frame, &mut world)).unwrap();
        let buffer = terminal.backend().buffer();

        let screen: Vec<String> = (0..16)
            .map(|y| (0..40).map(|x| buffer[(x, y)].symbol()).collect())
            .collect();
        let top = rows_containing(&screen, "top")[0];
        let longer = rows_containing(&screen, "longer line")[0];
        assert_eq!(longer, top + 1);

        let x = col_of(&screen[longer], "longer line") as u16;
        for y in [top as u16, longer as u16] {
            let reversed = buffer[(x, y)]
                .style()
                .add_modifier
                .contains(ratatui::style::Modifier::REVERSED);
            assert!(reversed, "row {y}");
        }
    }

    #[test]
    fn two_line_self_message_wraps_both_labels() {
        let screen = frame_screen("sequenceDiagram\nparticipant A\nA->>A: self<br>loop\n", 40);

        let bottom = screen.iter().position(|r| r.contains('◀')).unwrap();
        assert!(screen[bottom - 3].contains('┐'));
        assert!(screen[bottom - 2].contains("│  │self"));
        assert!(screen[bottom - 1].contains("│  │loop"));
    }

    #[test]
    fn autonumber_prefixes_only_first_line() {
        let screen = frame_screen(
            "sequenceDiagram\nautonumber\nparticipant A\nparticipant B\nA->>B: first<br/>second\n",
            40,
        );

        let first = rows_containing(&screen, "1. first")[0];
        assert!(screen[first + 1].contains("second"));
        assert!(!screen[first + 1].contains("1."));
    }

    #[test]
    fn frame_encloses_three_line_note() {
        let screen = frame_screen(
            "sequenceDiagram\nparticipant A\nloop\nNote over A: a<br/>b<br/>c\nend\n",
            40,
        );

        let header = rows_containing(&screen, "┌ loop")[0];
        let footer = rows_containing(&screen, "└")
            .into_iter()
            .find(|&r| r > header)
            .unwrap();
        let c_row = rows_containing(&screen, "c")
            .into_iter()
            .find(|&r| r > header)
            .unwrap();

        assert!(header < c_row && c_row < footer);
        assert_eq!(footer - header, 5);
    }

    #[test]
    fn alt_enter_inserts_line_break_in_text_input() {
        let mut world = sample_world();
        world.get_mut::<EditorState>().mode = EditorMode::InputMessage;
        type_text(&mut world, "a");
        handle_key(&mut world, KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT));
        type_text(&mut world, "b");

        assert_eq!(world.get::<EditorState>().input_buffer, "a<br/>b");
    }

    #[test]
    fn frame_encloses_self_message_loop() {
        let screen = frame_screen(
            "sequenceDiagram\nparticipant A\nparticipant B\nloop\nA->>A: think hard\nend\n",
            60,
        );

        let message = screen.iter().find(|r| r.contains("think hard")).unwrap();
        let header = screen.iter().find(|r| r.contains("┌ loop")).unwrap();
        let frame_right = col_of(header, "┐");
        let text_end = col_of(message, "think hard") + "think hard".len();

        assert!(text_end < frame_right);
        assert_eq!(message.chars().nth(frame_right), Some('│'));
        assert!(message.contains("think hard"));
    }

    #[test]
    fn hash_toggles_autonumber_and_undo_restores() {
        let mut world = sample_world();

        press_char(&mut world, '#');
        assert!(world.get::<SequenceDiagram>().autonumber);
        assert_eq!(status(&world), "Autonumber on");
        assert!(is_dirty(&world));

        press_char(&mut world, '#');
        assert!(!world.get::<SequenceDiagram>().autonumber);
        assert_eq!(status(&world), "Autonumber off");

        press_char(&mut world, 'u');
        assert!(world.get::<SequenceDiagram>().autonumber);
        press_char(&mut world, 'u');
        assert!(!world.get::<SequenceDiagram>().autonumber);
        assert!(!is_dirty(&world));
    }

    #[test]
    fn shifted_hash_also_toggles_autonumber() {
        let mut world = sample_world();

        let event = KeyEvent::new(KeyCode::Char('#'), KeyModifiers::SHIFT);
        handle_key(&mut world, event);

        assert!(world.get::<SequenceDiagram>().autonumber);
    }

    #[test]
    fn shifted_question_mark_opens_help() {
        let mut world = sample_world();

        let event = KeyEvent::new(KeyCode::Char('?'), KeyModifiers::SHIFT);
        handle_key(&mut world, event);

        assert_eq!(world.get::<EditorState>().mode, EditorMode::Help);
    }

    #[test]
    fn autonumber_prefixes_messages_but_not_notes() {
        let screen = frame_screen(
            "sequenceDiagram\nautonumber\nparticipant A\nparticipant B\nA->>B: hi\nNote over A: n\nB->>B: me\n",
            60,
        );

        assert!(screen.iter().any(|r| r.contains("1. hi")));
        assert!(screen.iter().any(|r| r.contains("2. me")));
        assert!(!screen.iter().any(|r| r.contains(". n")));
    }

    #[test]
    fn frame_encloses_numbered_self_message() {
        let screen = frame_screen(
            "sequenceDiagram\nautonumber\nparticipant A\nparticipant B\nloop\nA->>A: think hard\nend\n",
            60,
        );

        let message = screen.iter().find(|r| r.contains("1. think hard")).unwrap();
        let header = screen.iter().find(|r| r.contains("┌ loop")).unwrap();
        let frame_right = col_of(header, "┐");
        let text_end = col_of(message, "1. think hard") + "1. think hard".len();

        assert!(text_end < frame_right);
        assert_eq!(message.chars().nth(frame_right), Some('│'));
    }

    fn world_selecting(source: &str, idx: usize) -> World {
        let diagram = SequenceDiagram::from_mermaid(source).unwrap();
        let mut world = world_with(diagram, None);
        world.get_mut::<EditorState>().selection = Selection::Event(idx);
        world
    }

    #[test]
    fn plus_and_minus_do_nothing_on_a_message() {
        let mut world = world_selecting("sequenceDiagram\nA->>B: hi\n", 0);
        let before = world.get::<SequenceDiagram>().clone();

        press_char(&mut world, '+');
        press_char(&mut world, '-');

        assert!(*world.get::<SequenceDiagram>() == before);
        assert!(!is_dirty(&world));
    }

    const FRAME_MOVE_SOURCE: &str = "sequenceDiagram\nparticipant A\nparticipant B\nA->>B: a\nopt x\nA->>B: b\nA->>B: c\nend\nA->>B: d\n";

    fn world_selecting_frame(source: &str) -> World {
        let diagram = SequenceDiagram::from_mermaid(source).unwrap();
        let mut world = world_with(diagram, None);
        world.get_mut::<EditorState>().selection = Selection::Fragment(0);
        world
    }

    fn event_texts(world: &World) -> Vec<String> {
        world
            .get::<SequenceDiagram>()
            .events
            .iter()
            .map(|e| e.text().to_string())
            .collect()
    }

    fn selection(world: &World) -> Selection {
        world.get::<EditorState>().selection
    }

    #[test]
    fn shift_j_and_k_move_selected_frame_and_selection_follows() {
        let mut world = world_selecting_frame(FRAME_MOVE_SOURCE);

        press_char(&mut world, 'J');
        assert_eq!(event_texts(&world), ["a", "d", "b", "c"]);
        assert_eq!(selection(&world), Selection::Fragment(0));
        assert_eq!(frames(&world)[0].2, 2);

        press_char(&mut world, 'K');
        assert_eq!(event_texts(&world), ["a", "b", "c", "d"]);
        press_char(&mut world, 'K');
        assert_eq!(event_texts(&world), ["b", "c", "a", "d"]);
        press_char(&mut world, 'K');
        assert_eq!(event_texts(&world), ["b", "c", "a", "d"]);
        assert_eq!(selection(&world), Selection::Fragment(0));
    }

    #[test]
    fn frame_move_undo_restores_and_clamps() {
        let mut world = world_selecting_frame(FRAME_MOVE_SOURCE);

        press_char(&mut world, 'J');
        press_char(&mut world, 'u');

        assert_eq!(event_texts(&world), ["a", "b", "c", "d"]);
        assert_eq!(selection(&world), Selection::Fragment(0));

        press_char(&mut world, 'J');
    }

    #[test]
    fn d_on_frame_keeps_events_and_selects_first_event() {
        let mut world = world_selecting_frame(FRAME_MOVE_SOURCE);

        press_char(&mut world, 'D');

        assert_eq!(frames(&world).len(), 0);
        assert_eq!(event_texts(&world).len(), 4);
        assert_eq!(selection(&world), Selection::Event(1));
        assert_eq!(status(&world), "Frame removed");
    }

    #[test]
    fn capital_f_on_frame_removes_only_the_frame() {
        let mut world = world_selecting_frame(FRAME_MOVE_SOURCE);

        press_char(&mut world, 'F');

        assert_eq!(frames(&world).len(), 0);
        assert_eq!(event_texts(&world).len(), 4);
        assert_eq!(selection(&world), Selection::Event(1));
    }

    #[test]
    fn enter_on_frame_edits_kind_and_label() {
        let mut world = world_selecting_frame(FRAME_MOVE_SOURCE);

        press(&mut world, KeyCode::Enter);
        assert_eq!(world.get::<EditorState>().frame_kind, FragmentKind::Opt);
        press(&mut world, KeyCode::Down);
        press(&mut world, KeyCode::Enter);
        assert_eq!(world.get::<EditorState>().input_buffer, "x");
        type_text(&mut world, "y");
        press(&mut world, KeyCode::Enter);

        let edited = frames(&world);
        assert_eq!(edited[0].1, "xy");
        assert_ne!(edited[0].0, FragmentKind::Opt);
        assert_eq!(selection(&world), Selection::Fragment(0));
    }

    fn insert_message_via_keys(world: &mut World, key: char, text: &str) {
        press_char(world, key);
        press(world, KeyCode::Enter);
        press(world, KeyCode::Enter);
        type_text(world, text);
        press(world, KeyCode::Enter);
    }

    #[test]
    fn m_on_frame_inserts_inside_and_capital_m_above() {
        let mut world = world_selecting_frame(FRAME_MOVE_SOURCE);
        insert_message_via_keys(&mut world, 'm', "in");
        assert_eq!(event_texts(&world), ["a", "in", "b", "c", "d"]);
        assert_eq!(frames(&world)[0].2..=frames(&world)[0].3, 1..=3);
        assert_eq!(selection(&world), Selection::Event(1));

        let mut world = world_selecting_frame(FRAME_MOVE_SOURCE);
        insert_message_via_keys(&mut world, 'M', "out");
        assert_eq!(event_texts(&world), ["a", "out", "b", "c", "d"]);
        assert_eq!(frames(&world)[0].2..=frames(&world)[0].3, 2..=3);
        assert_eq!(selection(&world), Selection::Event(1));
    }

    #[test]
    fn frame_selection_ignores_unrelated_keys() {
        let mut world = world_selecting_frame(FRAME_MOVE_SOURCE);
        let before = world.get::<SequenceDiagram>().clone();

        for c in ['H', 'L', 'a', 'A', 't'] {
            press_char(&mut world, c);
        }

        assert!(*world.get::<SequenceDiagram>() == before);
        assert_eq!(selection(&world), Selection::Fragment(0));
    }

    #[test]
    fn selected_frame_header_uses_selected_style() {
        let mut world = world_selecting_frame(FRAME_MOVE_SOURCE);
        let selected = world.get::<Theme>().selected;
        let muted = world.get::<Theme>().muted;

        let mut terminal = Terminal::new(TestBackend::new(60, 24)).unwrap();
        terminal.draw(|frame| render(frame, &mut world)).unwrap();
        let buffer = terminal.backend().buffer();
        let rows: Vec<String> = (0..24)
            .map(|y| (0..60).map(|x| buffer[(x, y)].symbol()).collect())
            .collect();
        let y = rows.iter().position(|r| r.contains("┌ opt")).unwrap();
        let x = col_of(&rows[y], "┌");
        let (x, y) = (x as u16, y as u16);

        assert_eq!(buffer[(x, y)].style().fg, selected.fg);
        assert_ne!(buffer[(x, y)].style().fg, muted.fg);
    }

    #[test]
    fn selecting_frame_below_viewport_scrolls_it_into_view() {
        let messages = "A->>B: m\n".repeat(12);
        let source = format!(
            "sequenceDiagram\nparticipant A\nparticipant B\n{messages}opt late\nA->>B: framed\nend\n"
        );
        let mut world = world_selecting_frame(&source);

        let mut terminal = Terminal::new(TestBackend::new(60, 16)).unwrap();
        terminal.draw(|frame| render(frame, &mut world)).unwrap();

        let buffer = terminal.backend().buffer();
        let screen: String = buffer
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(screen.contains("framed"));
        assert!(world.get::<ScrollState>().offset > 0);
    }

    #[test]
    fn undo_of_frame_creation_selects_its_first_event_in_view() {
        let messages = "A->>B: m\n".repeat(20);
        let source =
            format!("sequenceDiagram\nparticipant A\nparticipant B\n{messages}A->>B: last\n");
        let diagram = SequenceDiagram::from_mermaid(&source).unwrap();
        let mut world = world_with(diagram, None);
        world.get_mut::<EditorState>().selection = Selection::Event(20);
        add_frame_via_keys(&mut world, "");
        press_char(&mut world, 'k');
        assert_eq!(selection(&world), Selection::Fragment(0));

        press_char(&mut world, 'u');

        assert_eq!(selection(&world), Selection::Event(20));
        let mut terminal = Terminal::new(TestBackend::new(60, 16)).unwrap();
        terminal.draw(|frame| render(frame, &mut world)).unwrap();
        let buffer = terminal.backend().buffer();
        let screen: String = buffer
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(screen.contains("last"));
    }

    #[test]
    fn undo_of_frame_swap_keeps_selection_on_same_frame() {
        let mut world = world_selecting_frame(
            "sequenceDiagram\nA->>B: 0\nopt a\nA->>B: 1\nend\nloop b\nA->>B: 2\nend\n",
        );
        world.get_mut::<EditorState>().selection = Selection::Fragment(0);

        press_char(&mut world, 'J');
        assert_eq!(selection(&world), Selection::Fragment(1));
        press_char(&mut world, 'u');

        assert_eq!(selection(&world), Selection::Fragment(0));
        assert_eq!(frames(&world)[0].1, "a");
    }

    #[test]
    fn undo_of_new_frame_restores_cursor_in_view() {
        let messages = "A->>B: m\n".repeat(20);
        let source = format!(
            "sequenceDiagram\nparticipant A\nparticipant B\nloop\nA->>B: top\nend\n{messages}"
        );
        let diagram = SequenceDiagram::from_mermaid(&source).unwrap();
        let mut world = world_with(diagram, None);
        world.get_mut::<EditorState>().selection = Selection::Event(16);
        add_frame_via_keys(&mut world, "");
        press_char(&mut world, 'k');
        assert_eq!(selection(&world), Selection::Fragment(1));

        press_char(&mut world, 'u');

        assert_eq!(selection(&world), Selection::Event(16));
    }

    #[test]
    fn undo_of_label_edit_keeps_frame_selected() {
        let mut world = world_selecting_frame(FRAME_MOVE_SOURCE);

        press(&mut world, KeyCode::Enter);
        press(&mut world, KeyCode::Enter);
        type_text(&mut world, "y");
        press(&mut world, KeyCode::Enter);
        assert_eq!(frames(&world)[0].1, "xy");
        press_char(&mut world, 'u');

        assert_eq!(frames(&world)[0].1, "x");
        assert_eq!(selection(&world), Selection::Fragment(0));
    }

    #[test]
    fn undo_returns_cursor_to_moved_events_original_position() {
        let mut world = world_selecting("sequenceDiagram\nA->>B: 0\nA->>B: 1\nA->>B: 2\n", 0);

        press_char(&mut world, 'J');
        assert_eq!(selection(&world), Selection::Event(1));
        press_char(&mut world, 'j');
        assert_eq!(selection(&world), Selection::Event(2));
        press_char(&mut world, 'u');

        assert_eq!(event_texts(&world), ["0", "1", "2"]);
        assert_eq!(selection(&world), Selection::Event(0));

        press_char(&mut world, 'U');
        assert_eq!(event_texts(&world), ["1", "0", "2"]);
        assert_eq!(selection(&world), Selection::Event(2));
    }

    const BOX_SOURCE: &str = "sequenceDiagram
    box Blue Frontend
        participant A
        participant B
    end
    participant C
";

    fn world_selecting_box(source: &str) -> World {
        let diagram = SequenceDiagram::from_mermaid(source).unwrap();
        let mut world = world_with(diagram, None);
        world.get_mut::<EditorState>().selection = Selection::Box(0);
        world
    }

    fn participant_ids(world: &World) -> Vec<String> {
        let diagram = world.get::<SequenceDiagram>();
        diagram.participants.iter().map(|p| p.id.clone()).collect()
    }

    fn box_range(world: &World, idx: usize) -> (usize, usize) {
        let b = &world.get::<SequenceDiagram>().boxes[idx];
        (b.start, b.end)
    }

    #[test]
    fn shift_l_and_h_move_box_and_selection_follows() {
        let mut world = world_selecting_box(BOX_SOURCE);

        press_char(&mut world, 'L');
        assert_eq!(participant_ids(&world), ["C", "A", "B"]);
        assert_eq!(box_range(&world, 0), (1, 2));
        assert_eq!(selection(&world), Selection::Box(0));

        press_char(&mut world, 'L');
        assert_eq!(box_range(&world, 0), (1, 2));

        press_char(&mut world, 'H');
        assert_eq!(participant_ids(&world), ["A", "B", "C"]);
        assert_eq!(box_range(&world, 0), (0, 1));

        press_char(&mut world, 'H');
        assert_eq!(box_range(&world, 0), (0, 1));
    }

    #[test]
    fn undo_after_box_move_restores_box_selection() {
        let mut world = world_selecting_box(BOX_SOURCE);

        press_char(&mut world, 'L');
        press_char(&mut world, 'u');

        assert_eq!(participant_ids(&world), ["A", "B", "C"]);
        assert_eq!(selection(&world), Selection::Box(0));
    }

    #[test]
    fn d_and_b_on_box_remove_only_the_box() {
        for key in ['D', 'B'] {
            let mut world = world_selecting_box(BOX_SOURCE);

            press_char(&mut world, key);

            assert_eq!(world.get::<SequenceDiagram>().boxes.len(), 0);
            assert_eq!(participant_ids(&world), ["A", "B", "C"]);
            assert_eq!(selection(&world), Selection::Participant(0));
            assert_eq!(status(&world), "Box removed");
        }
    }

    #[test]
    fn enter_on_colored_box_edits_color_then_label() {
        let mut world = world_selecting_box(BOX_SOURCE);

        press(&mut world, KeyCode::Enter);
        assert_eq!(world.get::<EditorState>().mode, EditorMode::SelectBoxColor);
        assert_eq!(world.get::<EditorState>().box_color, BoxColor::Blue);

        press_char(&mut world, 'j');
        press(&mut world, KeyCode::Enter);
        assert_eq!(world.get::<EditorState>().mode, EditorMode::InputBoxLabel);
        assert_eq!(world.get::<EditorState>().input_buffer, "Frontend");

        type_text(&mut world, "2");
        press(&mut world, KeyCode::Enter);

        let b = &world.get::<SequenceDiagram>().boxes[0];
        assert_eq!(b.label, "Frontend2");
        assert_eq!(b.color, Some(BoxColor::Green));
        assert_eq!(world.get::<EditorState>().mode, EditorMode::Normal);
    }

    #[test]
    fn enter_on_bare_box_opens_picker_and_sets_color() {
        let source = "sequenceDiagram\nbox\nparticipant A\nend\n";
        let mut world = world_selecting_box(source);

        press_char(&mut world, 'r');
        assert_eq!(world.get::<EditorState>().mode, EditorMode::SelectBoxColor);
        assert_eq!(world.get::<EditorState>().box_color, DEFAULT_BOX_COLOR);

        press_char(&mut world, 'j');
        press(&mut world, KeyCode::Enter);
        press(&mut world, KeyCode::Enter);

        let b = &world.get::<SequenceDiagram>().boxes[0];
        assert_eq!(b.color, Some(BoxColor::Blue));
        assert_eq!(b.label, "");
    }

    #[test]
    fn enter_on_rgb_box_edits_label_only_and_keeps_no_color() {
        let source = "sequenceDiagram\nbox rgb(10, 20, 30) Foo\nparticipant A\nend\n";
        let mut world = world_selecting_box(source);

        press(&mut world, KeyCode::Enter);
        assert_eq!(world.get::<EditorState>().mode, EditorMode::InputBoxLabel);

        type_text(&mut world, "!");
        press(&mut world, KeyCode::Enter);

        let b = &world.get::<SequenceDiagram>().boxes[0];
        assert_eq!(b.label, "rgb(10, 20, 30) Foo!");
        assert_eq!(b.color, None);
    }

    #[test]
    fn p_on_box_adds_participant_inside_and_keeps_selection() {
        let mut world = world_selecting_box(BOX_SOURCE);

        press_char(&mut world, 'p');
        type_text(&mut world, "Z");
        press(&mut world, KeyCode::Enter);

        assert_eq!(participant_ids(&world), ["A", "B", "Z", "C"]);
        assert_eq!(box_range(&world, 0), (0, 2));
        assert_eq!(selection(&world), Selection::Box(0));
    }

    #[test]
    fn j_from_box_goes_to_event_and_k_returns_to_its_first_participant() {
        let source =
            "sequenceDiagram\nparticipant X\nbox\nparticipant A\nend\nparticipant B\nA->>B: hi\n";
        let mut world = world_selecting_box(source);

        press_char(&mut world, 'j');
        assert_eq!(selection(&world), Selection::Event(0));

        press_char(&mut world, 'k');
        assert_eq!(selection(&world), Selection::Participant(1));
    }

    fn render_rows(world: &mut World, width: u16, height: u16) -> Vec<String> {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| render(frame, world)).unwrap();
        let buffer = terminal.backend().buffer();
        (0..height)
            .map(|y| (0..width).map(|x| buffer[(x, y)].symbol()).collect())
            .collect()
    }

    #[test]
    fn box_renders_as_outline_without_background() {
        let mut world = sample_world_with_box();
        let mut terminal = Terminal::new(TestBackend::new(60, 20)).unwrap();
        terminal.draw(|frame| render(frame, &mut world)).unwrap();
        let buffer = terminal.backend().buffer();

        let rows: Vec<String> = (0..20)
            .map(|y| (0..60).map(|x| buffer[(x, y)].symbol()).collect())
            .collect();
        assert!(rows.iter().any(|r| r.contains("╭─ Frontend")));
        assert!(rows.iter().any(|r| r.contains('╰') && r.contains('╯')));

        let top = rows.iter().position(|r| r.contains('╭')).unwrap();
        let left = col_of(&rows[top], "╭") as u16;
        let inside = (left + 1, top as u16 + 5);
        let outside = (58, top as u16 + 5);
        assert_eq!(buffer[inside].style().bg, buffer[outside].style().bg);
    }

    fn sample_world_with_box() -> World {
        let diagram = SequenceDiagram::from_mermaid(
            "sequenceDiagram\nbox Blue Frontend\nparticipant A\nparticipant B\nend\nparticipant C\nA->>C: hi\n",
        )
        .unwrap();
        world_with(diagram, None)
    }

    #[test]
    fn selected_box_outline_and_label_use_selected_style() {
        let mut world = sample_world_with_box();
        world.get_mut::<EditorState>().selection = Selection::Box(0);
        let selected = world.get::<Theme>().selected;

        let mut terminal = Terminal::new(TestBackend::new(60, 20)).unwrap();
        terminal.draw(|frame| render(frame, &mut world)).unwrap();
        let buffer = terminal.backend().buffer();
        let rows: Vec<String> = (0..20)
            .map(|y| (0..60).map(|x| buffer[(x, y)].symbol()).collect())
            .collect();

        let top = rows.iter().position(|r| r.contains('╭')).unwrap();
        let left = col_of(&rows[top], "╭") as u16;
        let label = col_of(&rows[top], "Frontend") as u16;
        assert_eq!(buffer[(left, top as u16)].style().fg, selected.fg);
        assert_eq!(buffer[(label, top as u16)].style().fg, selected.fg);
    }

    #[test]
    fn selected_empty_label_box_still_highlights_top_edge() {
        let diagram = SequenceDiagram::from_mermaid(
            "sequenceDiagram\nbox\nparticipant A\nend\nparticipant B\nA->>B: hi\n",
        )
        .unwrap();
        let mut world = world_with(diagram, None);
        world.get_mut::<EditorState>().selection = Selection::Box(0);
        let selected = world.get::<Theme>().selected;

        let mut terminal = Terminal::new(TestBackend::new(60, 20)).unwrap();
        terminal.draw(|frame| render(frame, &mut world)).unwrap();
        let buffer = terminal.backend().buffer();
        let rows: Vec<String> = (0..20)
            .map(|y| (0..60).map(|x| buffer[(x, y)].symbol()).collect())
            .collect();

        let top = rows.iter().position(|r| r.contains('╭')).unwrap();
        let left = col_of(&rows[top], "╭") as u16;
        assert_eq!(buffer[(left, top as u16)].style().fg, selected.fg);
    }

    #[test]
    fn adjacent_boxes_keep_their_own_corners() {
        let diagram = SequenceDiagram::from_mermaid(
            "sequenceDiagram\nbox One\nparticipant A\nend\nbox Two\nparticipant B\nend\n",
        )
        .unwrap();
        let mut world = world_with(diagram, None);

        let rows = render_rows(&mut world, 40, 12);
        let top = rows.iter().find(|r| r.contains('╭')).unwrap();

        assert_eq!(top.matches('╭').count(), 2, "{top}");
        assert_eq!(top.matches('╮').count(), 2, "{top}");
        assert!(rows.iter().any(|r| r.matches('╰').count() == 2));
    }

    #[test]
    fn boxes_render_on_narrow_terminals() {
        for width in [3, 8, 20] {
            let mut world = sample_world_with_box();
            world.get_mut::<EditorState>().selection = Selection::Box(0);
            render_rows(&mut world, width, 14);
        }
    }

    #[test]
    fn single_participant_box_widens_to_show_its_label() {
        let diagram = SequenceDiagram::from_mermaid(
            "sequenceDiagram\nbox Red Backend\nparticipant A\nend\nparticipant B\nA->>B: hi\n",
        )
        .unwrap();
        let mut world = world_with(diagram, None);

        let rows = render_rows(&mut world, 100, 24);

        assert!(rows.iter().any(|r| r.contains("╭─ Backend ")), "{rows:?}");
    }

    #[test]
    fn box_border_leaves_a_blank_column_before_headers() {
        let mut world = sample_world_with_box();

        let rows = render_rows(&mut world, 60, 20);
        let top = rows.iter().position(|r| r.contains('╭')).unwrap();
        let left = col_of(&rows[top], "╭");
        let header: Vec<char> = rows[top + 1].chars().collect();

        assert_eq!(header[left], '│');
        assert_eq!(header[left + 1], ' ');
        assert_eq!(header[left + 2], '┌');
    }

    #[test]
    fn tiny_terminals_render_without_panic() {
        for height in [1, 2] {
            let mut world = sample_world_with_box();
            render_rows(&mut world, 10, height);
        }
    }

    #[test]
    fn confirming_default_picker_color_keeps_bare_box_uncolored() {
        let source = "sequenceDiagram\nbox\nparticipant A\nend\n";
        let mut world = world_selecting_box(source);

        press(&mut world, KeyCode::Enter);
        press(&mut world, KeyCode::Enter);
        press(&mut world, KeyCode::Enter);

        assert_eq!(world.get::<SequenceDiagram>().boxes[0].color, None);
    }

    fn wide_world(messages: &str) -> World {
        let names = (1..=8)
            .map(|i| format!("participant Participant{i}\n"))
            .collect::<Vec<String>>()
            .concat();
        let source = format!("sequenceDiagram\n{names}{messages}");
        world_with(SequenceDiagram::from_mermaid(&source).unwrap(), None)
    }

    fn x_offset(world: &World) -> u16 {
        world.get::<ScrollState>().x_offset
    }

    #[test]
    fn selecting_participants_scrolls_wide_diagram_sideways() {
        let mut world = wide_world("");
        press_char(&mut world, 'l');

        let rows = render_rows(&mut world, 80, 24);
        assert!(!rows[3].contains('‹'));
        assert!(rows[3].ends_with('›'), "{}", rows[3]);

        for _ in 0..7 {
            press_char(&mut world, 'l');
            render_rows(&mut world, 80, 24);
        }
        let rows = render_rows(&mut world, 80, 24);
        assert_eq!(selection(&world), Selection::Participant(7));
        assert!(rows[3].contains("Participant8"), "{}", rows[3]);
        assert!(rows[3].starts_with('‹'), "{}", rows[3]);
        assert!(x_offset(&world) > 0);

        for _ in 0..7 {
            press_char(&mut world, 'h');
            render_rows(&mut world, 80, 24);
        }
        assert_eq!(selection(&world), Selection::Participant(0));
        assert_eq!(x_offset(&world), 0);
    }

    #[test]
    fn selected_message_between_last_participants_shows_its_label() {
        let mut world = wide_world("Participant7->>Participant8: far right label\n");
        press_char(&mut world, 'g');

        let rows = render_rows(&mut world, 80, 24);

        assert!(rows.iter().any(|r| r.contains("far right label")));
    }

    #[test]
    fn wide_diagram_with_box_and_frame_clips_without_damaging_borders() {
        let mut world = wide_world(
            "loop retry\nParticipant4->>Participant6: first message\nend\nParticipant8->>Participant8: self\n",
        );
        world
            .get_mut::<SequenceDiagram>()
            .add_box("Group".to_string(), BoxColor::Blue, 1, 4);

        for width in [3, 8, 20, 37, 60, 80] {
            let selections = [
                Selection::Participant(0),
                Selection::Participant(7),
                Selection::Box(0),
                Selection::Fragment(0),
                Selection::Event(0),
                Selection::Event(1),
            ];
            for selection in selections {
                world.get_mut::<EditorState>().selection = selection;
                let rows = render_rows(&mut world, width, 24);

                if width < 3 {
                    continue;
                }
                let middle: Vec<char> = rows[12].chars().collect();
                assert_eq!(middle[0], '│', "{selection:?} at width {width}");
                assert_eq!(
                    middle[usize::from(width) - 1],
                    '│',
                    "{selection:?} at width {width}"
                );
            }
        }
    }

    #[test]
    fn g_and_shift_g_jump_to_first_and_last_event() {
        let mut world = framed_world();
        press_char(&mut world, 'G');
        assert_eq!(selection(&world), Selection::Event(2));

        press_char(&mut world, 'g');
        assert_eq!(selection(&world), Selection::Event(0));
    }

    #[test]
    fn g_selects_frame_header_when_first_event_starts_a_frame() {
        let mut world = framed_world();
        world
            .get_mut::<SequenceDiagram>()
            .add_fragment(FragmentKind::Loop, String::new(), 0, 1);
        world.get_mut::<EditorState>().selection = Selection::Event(2);

        press_char(&mut world, 'g');

        assert_eq!(selection(&world), Selection::Fragment(0));
    }

    #[test]
    fn g_and_shift_g_do_nothing_without_events() {
        let diagram = SequenceDiagram::from_mermaid("sequenceDiagram\nparticipant A\n").unwrap();
        let mut world = world_with(diagram, None);
        world.get_mut::<EditorState>().selection = Selection::Participant(0);

        press_char(&mut world, 'g');
        press_char(&mut world, 'G');

        assert_eq!(selection(&world), Selection::Participant(0));
    }

    #[test]
    fn shift_g_scrolls_long_diagram_vertically() {
        let messages = (1..=20)
            .map(|i| format!("A->>B: message {i}\n"))
            .collect::<Vec<String>>()
            .concat();
        let source = format!("sequenceDiagram\n{messages}");
        let mut world = world_with(SequenceDiagram::from_mermaid(&source).unwrap(), None);

        press_char(&mut world, 'G');
        let rows = render_rows(&mut world, 60, 16);

        assert!(rows.iter().any(|r| r.contains("message 20")));
        assert!(world.get::<ScrollState>().offset > 0);
    }

    #[test]
    fn scrolled_fully_right_leaves_blank_column_after_last_header() {
        let mut world = wide_world("");
        world.get_mut::<EditorState>().selection = Selection::Participant(7);

        let rows = render_rows(&mut world, 80, 24);
        let header: Vec<char> = rows[2].chars().collect();
        let corner = header.iter().rposition(|&c| c == '┐').unwrap();

        assert_eq!(header[corner + 1], ' ', "{}", rows[2]);
        assert_eq!(header[79], '│', "{}", rows[2]);
    }

    #[test]
    fn frame_around_left_note_of_first_participant_keeps_left_edge() {
        let mut world = world_with(
            SequenceDiagram::from_mermaid(
                "sequenceDiagram\nparticipant A\nparticipant B\nloop\nNote left of A: left note text\nend\n",
            )
            .unwrap(),
            None,
        );

        let rows = render_rows(&mut world, 40, 16);
        let note_row = rows.iter().find(|r| r.contains("left note text")).unwrap();
        let note_col = col_of(note_row, "left note text");
        let frame_row = rows.iter().find(|r| r.contains("┌ loop")).unwrap();
        let frame_col = col_of(frame_row, "┌");

        assert!(frame_col < note_col, "{rows:#?}");
        assert!(note_row.chars().nth(frame_col) == Some('│'), "{rows:#?}");
    }

    fn draw(world: &mut World, width: u16, height: u16) -> ratatui::buffer::Buffer {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| render(frame, world)).unwrap();
        terminal.backend().buffer().clone()
    }

    fn screen_text(buffer: &ratatui::buffer::Buffer) -> String {
        buffer
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect()
    }

    fn status_line(world: &mut World) -> String {
        let buffer = draw(world, 100, 24);
        (0..100).map(|x| buffer[(x, 23)].symbol()).collect()
    }

    /// Style of the first cell showing `symbol`.
    fn style_of(buffer: &ratatui::buffer::Buffer, symbol: &str) -> ratatui::style::Style {
        let cell = buffer
            .content()
            .iter()
            .find(|c| c.symbol() == symbol)
            .unwrap_or_else(|| panic!("no {symbol} on screen"));
        cell.style()
    }

    fn undo_len(world: &World) -> usize {
        world.get::<AppState>().undo_stack.len()
    }

    #[test]
    fn f_with_nothing_selected_starts_at_first_event() {
        let mut world = framed_world();
        world.get_mut::<EditorState>().selection = Selection::None;

        press_char(&mut world, 'f');

        assert_eq!(world.get::<EditorState>().mode, EditorMode::SelectFrameEnd);
        assert_eq!(world.get::<EditorState>().selection, Selection::Event(0));
        assert_eq!(world.get::<EditorState>().frame_start, Some(0));
    }

    #[test]
    fn f_with_participant_selected_starts_at_first_event() {
        let mut world = framed_world();
        world.get_mut::<EditorState>().selection = Selection::Participant(1);

        press_char(&mut world, 'f');

        assert_eq!(world.get::<EditorState>().mode, EditorMode::SelectFrameEnd);
        assert_eq!(world.get::<EditorState>().selection, Selection::Event(0));
    }

    #[test]
    fn f_without_events_shows_error() {
        let diagram = SequenceDiagram::from_mermaid("sequenceDiagram\n participant A\n").unwrap();
        let mut world = world_with(diagram, None);
        world.get_mut::<EditorState>().selection = Selection::None;

        press_char(&mut world, 'f');

        assert_eq!(world.get::<EditorState>().mode, EditorMode::Normal);
        assert_eq!(status(&world), "Add a message first (m)");
    }

    #[test]
    fn frame_preview_is_drawn_while_selecting_and_gone_after_escape() {
        let mut world = framed_world();
        world.get_mut::<EditorState>().selection = Selection::Event(0);
        let undo_before = undo_len(&world);
        let diagram_before = world.get::<SequenceDiagram>().to_mermaid();

        press_char(&mut world, 'f');
        press_char(&mut world, 'j');
        let screen = screen_text(&draw(&mut world, 80, 24));

        assert!(screen.contains("┌ new frame"), "{screen}");
        assert!(screen.contains("└┄") && screen.contains("┄┘"));
        assert_eq!(world.get::<SequenceDiagram>().fragments.len(), 0);

        press(&mut world, KeyCode::Esc);
        let screen = screen_text(&draw(&mut world, 80, 24));

        assert!(!screen.contains("new frame"));
        assert!(!screen.contains("└┄"));
        assert_eq!(world.get::<SequenceDiagram>().to_mermaid(), diagram_before);
        assert!(!is_dirty(&world));
        assert_eq!(undo_len(&world), undo_before);
    }

    #[test]
    fn frame_preview_grows_with_cursor() {
        let mut world = framed_world();
        world.get_mut::<EditorState>().selection = Selection::Event(0);

        press_char(&mut world, 'f');
        let one: Vec<String> = render_rows(&mut world, 80, 24);
        press_char(&mut world, 'j');
        let two: Vec<String> = render_rows(&mut world, 80, 24);

        let footer = |rows: &[String]| rows.iter().position(|r| r.contains("└┄")).unwrap();
        assert!(footer(&two) > footer(&one));
    }

    #[test]
    fn frame_preview_over_existing_frame_marks_it_red_with_hint() {
        let mut world = framed_world();
        world.get_mut::<SequenceDiagram>().add_fragment(
            FragmentKind::Loop,
            "Mark".to_string(),
            1,
            1,
        );
        world.get_mut::<EditorState>().selection = Selection::Event(0);
        press_char(&mut world, 'f');

        let ok = draw(&mut world, 100, 24);
        assert_eq!(style_of(&ok, "M").fg, world.get::<Theme>().accent.fg);
        assert!(!status_line(&mut world).contains("overlaps a frame"));

        press_char(&mut world, 'j');
        let buffer = draw(&mut world, 100, 24);

        let error = world.get::<Theme>().error;
        assert_eq!(style_of(&buffer, "M").fg, error.fg);
        assert!(!screen_text(&buffer).contains("new frame"));
        assert!(status_line(&mut world).contains("overlaps a frame"));
    }

    #[test]
    fn frame_mode_hints() {
        let mut world = framed_world();
        world.get_mut::<EditorState>().selection = Selection::Event(0);

        press_char(&mut world, 'f');
        assert!(
            status_line(&mut world)
                .contains("j/k: choose last message  Enter: pick kind  Esc: cancel")
        );

        press(&mut world, KeyCode::Enter);
        assert!(status_line(&mut world).contains("j/k: kind  Enter: next (label)  Esc: cancel"));

        press(&mut world, KeyCode::Enter);
        let screen = screen_text(&draw(&mut world, 100, 24));
        assert!(screen.contains("Label (optional, Enter to finish):"));
    }

    #[test]
    fn b_without_participants_shows_error() {
        let mut world = world_with(SequenceDiagram::new(), None);

        press_char(&mut world, 'b');

        assert_eq!(world.get::<EditorState>().mode, EditorMode::Normal);
        assert_eq!(status(&world), "Add a participant first (p)");
    }

    #[test]
    fn b_starts_at_selected_participant_else_first() {
        let mut world = sample_world();
        world.get_mut::<EditorState>().selection = Selection::Participant(1);
        press_char(&mut world, 'b');
        assert_eq!(world.get::<EditorState>().mode, EditorMode::SelectBoxEnd);
        assert_eq!(world.get::<EditorState>().box_start, Some(1));
        assert_eq!(selection(&world), Selection::Participant(1));
        press(&mut world, KeyCode::Esc);

        world.get_mut::<EditorState>().selection = Selection::None;
        press_char(&mut world, 'b');
        assert_eq!(world.get::<EditorState>().box_start, Some(0));
        assert_eq!(selection(&world), Selection::Participant(0));
        press(&mut world, KeyCode::Esc);

        world.get_mut::<EditorState>().selection = Selection::Event(0);
        press_char(&mut world, 'b');
        assert_eq!(world.get::<EditorState>().box_start, Some(0));
    }

    #[test]
    fn b_on_box_or_member_edits_that_box() {
        let mut world = sample_world_with_box();
        world.get_mut::<EditorState>().selection = Selection::Box(0);
        press_char(&mut world, 'b');
        assert_eq!(world.get::<EditorState>().mode, EditorMode::SelectBoxColor);
        assert_eq!(world.get::<EditorState>().box_start, None);
        press(&mut world, KeyCode::Esc);

        world.get_mut::<EditorState>().selection = Selection::Participant(1);
        press_char(&mut world, 'b');
        assert_eq!(world.get::<EditorState>().mode, EditorMode::SelectBoxColor);
        assert_eq!(selection(&world), Selection::Box(0));
    }

    #[test]
    fn b_with_nothing_selected_skips_boxed_participants() {
        let mut world = sample_world_with_box();
        world.get_mut::<EditorState>().selection = Selection::None;

        press_char(&mut world, 'b');

        assert_eq!(world.get::<EditorState>().box_start, Some(2));
    }

    #[test]
    fn b_with_every_participant_boxed_shows_error() {
        let mut world = sample_world();
        world
            .get_mut::<SequenceDiagram>()
            .add_box(String::new(), BoxColor::Red, 0, 1);
        world.get_mut::<EditorState>().selection = Selection::None;

        press_char(&mut world, 'b');

        assert_eq!(world.get::<EditorState>().mode, EditorMode::Normal);
        assert_eq!(status(&world), "No free participants to box");
    }

    #[test]
    fn f_with_nothing_selected_skips_framed_events() {
        let mut world = framed_world();
        world
            .get_mut::<SequenceDiagram>()
            .add_fragment(FragmentKind::Loop, String::new(), 0, 1);
        world.get_mut::<EditorState>().selection = Selection::None;

        press_char(&mut world, 'f');
        assert_eq!(world.get::<EditorState>().frame_start, Some(2));
        press(&mut world, KeyCode::Esc);

        world
            .get_mut::<SequenceDiagram>()
            .add_fragment(FragmentKind::Opt, String::new(), 2, 2);
        press_char(&mut world, 'f');
        assert_eq!(world.get::<EditorState>().mode, EditorMode::Normal);
        assert_eq!(status(&world), "No free messages to frame");
    }

    #[test]
    fn box_end_moves_only_with_h_and_l() {
        let mut world = sample_world();
        world.get_mut::<EditorState>().selection = Selection::Participant(0);
        press_char(&mut world, 'b');

        press_char(&mut world, 'j');
        press(&mut world, KeyCode::Down);
        assert_eq!(selection(&world), Selection::Participant(0));

        press_char(&mut world, 'l');
        assert_eq!(selection(&world), Selection::Participant(1));
        press_char(&mut world, 'l');
        assert_eq!(selection(&world), Selection::Participant(1));
        press(&mut world, KeyCode::Left);
        assert_eq!(selection(&world), Selection::Participant(0));
    }

    #[test]
    fn box_preview_grows_with_l_and_escape_restores_everything() {
        let mut world = sample_world();
        world.get_mut::<EditorState>().selection = Selection::Event(0);
        let undo_before = undo_len(&world);
        let diagram_before = world.get::<SequenceDiagram>().to_mermaid();

        press_char(&mut world, 'b');
        let one = render_rows(&mut world, 80, 24);
        press_char(&mut world, 'l');
        let two = render_rows(&mut world, 80, 24);
        let buffer = draw(&mut world, 80, 24);
        let screen = screen_text(&buffer);

        let width = |rows: &[String]| {
            let row = rows.iter().find(|r| r.contains("╭")).unwrap();
            row.chars().filter(|&c| c == '┄' || c == '─').count()
        };
        assert!(width(&two) > width(&one));
        assert!(screen.contains("new box"), "{screen}");
        assert!(screen.contains("╰┄") && screen.contains("┄╯"));
        assert_eq!(style_of(&buffer, "┄").fg, world.get::<Theme>().selected.fg);
        assert_eq!(world.get::<SequenceDiagram>().boxes.len(), 0);

        press(&mut world, KeyCode::Esc);
        let screen = screen_text(&draw(&mut world, 80, 24));

        assert!(!screen.contains("new box"));
        assert!(!screen.contains("╭┄"));
        assert_eq!(selection(&world), Selection::Event(0));
        assert_eq!(world.get::<EditorState>().mode, EditorMode::Normal);
        assert_eq!(world.get::<SequenceDiagram>().to_mermaid(), diagram_before);
        assert!(!is_dirty(&world));
        assert_eq!(undo_len(&world), undo_before);
    }

    #[test]
    fn box_selection_renders_no_participant_popup() {
        let mut world = sample_world();
        press_char(&mut world, 'b');

        let screen = screen_text(&draw(&mut world, 80, 24));

        assert!(!screen.contains("Box: "));
    }

    #[test]
    fn box_range_overlapping_existing_boxes_keeps_them_drawn_in_red() {
        let source = "sequenceDiagram\nparticipant A\nbox Aqua Mid\nparticipant B\nparticipant C\nend\nparticipant D\nA->>D: hi\n";
        // Partial overlap from either side, and a range covering the whole box.
        let cases = [(0, 'l', 1), (0, 'l', 3), (3, 'h', 1), (3, 'h', 3)];

        for (start, key, steps) in cases {
            let mut world = world_with(SequenceDiagram::from_mermaid(source).unwrap(), None);
            world.get_mut::<EditorState>().selection = Selection::Participant(start);
            press_char(&mut world, 'b');
            for _ in 0..steps {
                press_char(&mut world, key);
            }

            let buffer = draw(&mut world, 100, 24);
            let screen = screen_text(&buffer);
            assert!(screen.contains("Mid"), "{screen}");
            assert!(!screen.contains("new box"), "{screen}");
            assert_eq!(style_of(&buffer, "M").fg, world.get::<Theme>().error.fg);
            assert!(status_line(&mut world).contains("overlaps a box"));
        }
    }

    #[test]
    fn box_end_hint() {
        let mut world = sample_world();

        press_char(&mut world, 'b');

        assert!(
            status_line(&mut world)
                .contains("h/l: choose last participant  Enter: pick color  Esc: cancel")
        );
    }

    #[test]
    fn escape_after_f_restores_selection_from_before() {
        let mut world = framed_world();
        world.get_mut::<EditorState>().selection = Selection::Participant(1);

        press_char(&mut world, 'f');
        press_char(&mut world, 'j');
        press(&mut world, KeyCode::Esc);

        assert_eq!(selection(&world), Selection::Participant(1));
    }

    #[test]
    fn box_end_scroll_follows_the_cursor_participant() {
        let participants: Vec<String> = (0..10).map(|i| format!("participant Pp{i}")).collect();
        let source = format!("sequenceDiagram\n{}\n", participants.join("\n"));
        let mut world = world_with(SequenceDiagram::from_mermaid(&source).unwrap(), None);
        world.get_mut::<EditorState>().selection = Selection::Participant(0);

        press_char(&mut world, 'b');
        for _ in 0..6 {
            press_char(&mut world, 'l');
            draw(&mut world, 60, 24);
        }

        let screen = screen_text(&draw(&mut world, 60, 24));
        assert!(screen.contains("Pp6"), "{screen}");
    }

    #[test]
    fn box_preview_follows_picker_color_and_label() {
        let mut world = sample_world();
        world.get_mut::<EditorState>().selection = Selection::Participant(0);
        press_char(&mut world, 'b');
        press_char(&mut world, 'l');
        press(&mut world, KeyCode::Enter);

        let before = draw(&mut world, 80, 24);
        press_char(&mut world, 'j');
        let after = draw(&mut world, 80, 24);
        assert_ne!(style_of(&before, "╭").fg, style_of(&after, "╭").fg);
        assert_eq!(
            style_of(&after, "╭").fg,
            Some(box_color(world.get::<EditorState>().box_color))
        );
        assert!(screen_text(&after).contains("╭┄ new box"));

        press(&mut world, KeyCode::Enter);
        type_text(&mut world, "Mine");
        let screen = screen_text(&draw(&mut world, 80, 24));
        assert!(screen.contains("╭┄ Mine"), "{screen}");
        assert!(!screen.contains("new box"));
    }

    #[test]
    fn frame_preview_follows_picker_kind_and_label() {
        let mut world = framed_world();
        world.get_mut::<EditorState>().selection = Selection::Event(0);
        press_char(&mut world, 'f');
        press_char(&mut world, 'j');
        press(&mut world, KeyCode::Enter);

        let screen = screen_text(&draw(&mut world, 80, 24));
        assert!(screen.contains("┌ loop"), "{screen}");
        press_char(&mut world, 'k');
        let screen = screen_text(&draw(&mut world, 80, 24));
        assert!(screen.contains("┌ break"), "{screen}");

        press(&mut world, KeyCode::Enter);
        type_text(&mut world, "Why");
        let screen = screen_text(&draw(&mut world, 80, 24));
        assert!(screen.contains("break [Why]"), "{screen}");
    }

    #[test]
    fn escape_at_every_create_step_restores_selection() {
        for steps in 0..3 {
            let mut world = framed_world();
            world.get_mut::<EditorState>().selection = Selection::Participant(1);
            press_char(&mut world, 'f');
            for _ in 0..steps {
                press(&mut world, KeyCode::Enter);
            }
            press(&mut world, KeyCode::Esc);
            assert_eq!(world.get::<EditorState>().mode, EditorMode::Normal);
            assert_eq!(selection(&world), Selection::Participant(1));

            world.get_mut::<EditorState>().selection = Selection::Event(2);
            press_char(&mut world, 'b');
            press_char(&mut world, 'l');
            for _ in 0..steps {
                press(&mut world, KeyCode::Enter);
            }
            press(&mut world, KeyCode::Esc);
            assert_eq!(world.get::<EditorState>().mode, EditorMode::Normal);
            assert_eq!(selection(&world), Selection::Event(2));
        }
    }

    #[test]
    fn overlapping_box_range_highlights_headers_in_accent() {
        let mut world = sample_world_with_box();
        world.get_mut::<EditorState>().selection = Selection::Participant(2);
        press_char(&mut world, 'b');
        press_char(&mut world, 'h');

        let buffer = draw(&mut world, 100, 24);

        // Header of C (start of the range, not the cursor) is accented.
        assert_eq!(style_of(&buffer, "C").fg, world.get::<Theme>().accent.fg);
    }

    const SECTION_SOURCE: &str = "sequenceDiagram\nparticipant A\nparticipant B\nalt ok\nA->>B: one\nelse expired\nB->>A: two\nend\n";
    const THREE_EVENTS_IN_ALT: &str = "sequenceDiagram\nparticipant A\nparticipant B\nalt ok\nA->>B: 1\nA->>B: 2\nA->>B: 3\nend\n";
    const TWO_SECTIONS: &str = "sequenceDiagram\nparticipant A\nparticipant B\nalt ok\nA->>B: 1\nA->>B: 2\nelse b\nA->>B: 3\nA->>B: 4\nend\n";

    fn world_with_selection(source: &str, selection: Selection) -> World {
        let diagram = SequenceDiagram::from_mermaid(source).unwrap();
        let mut world = world_with(diagram, None);
        world.get_mut::<EditorState>().selection = selection;
        world
    }

    fn sections(world: &World) -> Vec<(usize, String)> {
        world.get::<SequenceDiagram>().fragments[0].sections.clone()
    }

    fn screen_row(buffer: &ratatui::buffer::Buffer, needle: &str) -> usize {
        let width = usize::from(buffer.area.width);
        let screen = screen_text(buffer);
        let pos = screen
            .find(needle)
            .unwrap_or_else(|| panic!("no {needle} on screen"));
        screen[..pos].chars().count() / width
    }

    #[test]
    fn divider_is_drawn_between_the_sections() {
        let mut world = world_with_selection(SECTION_SOURCE, Selection::None);
        let buffer = draw(&mut world, 80, 24);
        let screen = screen_text(&buffer);

        assert!(screen.contains("├┄ [expired] ┄"), "{screen}");
        assert!(screen.contains('┤'), "{screen}");
        let divider = screen_row(&buffer, "├┄ [expired]");
        assert!(screen_row(&buffer, "one") < divider);
        assert_eq!(screen_row(&buffer, "two"), divider + 1);
    }

    #[test]
    fn divider_without_label_is_all_dashes() {
        let source = SECTION_SOURCE.replace("else expired", "else");
        let mut world = world_with_selection(&source, Selection::None);
        let screen = screen_text(&draw(&mut world, 80, 24));

        assert!(screen.contains("├┄┄┄┄"), "{screen}");
        assert!(!screen.contains("[]"), "{screen}");
    }

    #[test]
    fn long_divider_label_is_truncated_in_narrow_terminal() {
        let source = SECTION_SOURCE.replace("expired", "a very long divider label that cannot fit");
        let mut world = world_with_selection(&source, Selection::None);
        let screen = screen_text(&draw(&mut world, 40, 24));

        assert!(screen.contains("├┄ [a very"), "{screen}");
        assert!(screen.contains("…]"), "{screen}");
    }

    #[test]
    fn selected_divider_is_highlighted() {
        let mut world = world_with_selection(SECTION_SOURCE, Selection::Section(0, 0));
        let buffer = draw(&mut world, 80, 24);

        assert_eq!(style_of(&buffer, "├").fg, world.get::<Theme>().selected.fg);
    }

    #[test]
    fn s_previews_label_and_enter_adds_and_selects_divider() {
        let mut world = world_with_selection(THREE_EVENTS_IN_ALT, Selection::Event(1));

        press_char(&mut world, 's');
        assert_eq!(
            world.get::<EditorState>().mode,
            EditorMode::InputSectionLabel
        );
        let screen = screen_text(&draw(&mut world, 80, 40));
        assert!(screen.contains("├┄┄┄"), "{screen}");

        type_text(&mut world, "late");
        let screen = screen_text(&draw(&mut world, 80, 40));
        assert!(screen.contains("├┄ [late]"), "{screen}");
        assert_eq!(sections(&world).len(), 0);

        press(&mut world, KeyCode::Enter);
        assert_eq!(sections(&world), [(1, "late".to_string())]);
        assert_eq!(selection(&world), Selection::Section(0, 0));
        assert_eq!(world.get::<EditorState>().mode, EditorMode::Normal);

        press_char(&mut world, 'u');
        assert_eq!(sections(&world).len(), 0);
        assert_eq!(selection(&world), Selection::Event(1));
    }

    #[test]
    fn escape_in_section_prompt_restores_selection() {
        let mut world = world_with_selection(THREE_EVENTS_IN_ALT, Selection::Event(2));
        press_char(&mut world, 's');
        type_text(&mut world, "x");
        press(&mut world, KeyCode::Esc);

        assert_eq!(world.get::<EditorState>().mode, EditorMode::Normal);
        assert_eq!(selection(&world), Selection::Event(2));
        assert_eq!(sections(&world).len(), 0);
        assert_eq!(undo_len(&world), 0);
    }

    #[test]
    fn s_reports_why_no_section_can_start() {
        let outside = "sequenceDiagram\nparticipant A\nparticipant B\nA->>B: x\nalt a\nA->>B: 1\nA->>B: 2\nend\nloop\nA->>B: 3\nA->>B: 4\nend\n";
        let cases = [
            (
                Selection::Event(0),
                "Select an event inside alt, par or critical",
            ),
            (
                Selection::None,
                "Select an event inside alt, par or critical",
            ),
            (
                Selection::Participant(0),
                "Select an event inside alt, par or critical",
            ),
            (
                Selection::Fragment(0),
                "Select an event inside alt, par or critical",
            ),
            (
                Selection::Event(1),
                "Pick an event below the frame's first one",
            ),
            (Selection::Event(4), "'loop' has no sections"),
        ];
        for (sel, message) in cases {
            let mut world = world_with_selection(outside, sel);
            press_char(&mut world, 's');
            assert_eq!(status(&world), message, "{sel:?}");
            assert_eq!(world.get::<EditorState>().mode, EditorMode::Normal);
        }

        let mut world = world_with_selection(SECTION_SOURCE, Selection::Event(1));
        press_char(&mut world, 's');
        assert_eq!(status(&world), "A section already starts here");

        let mut world = world_with_selection(SECTION_SOURCE, Selection::Section(0, 0));
        press_char(&mut world, 's');
        assert_eq!(status(&world), "A section already starts here");
    }

    #[test]
    fn enter_on_divider_edits_its_label() {
        let mut world = world_with_selection(SECTION_SOURCE, Selection::Section(0, 0));

        press(&mut world, KeyCode::Enter);
        assert_eq!(world.get::<EditorState>().input_buffer, "expired");
        type_text(&mut world, "!");
        press(&mut world, KeyCode::Enter);
        assert_eq!(sections(&world), [(1, "expired!".to_string())]);
        assert_eq!(selection(&world), Selection::Section(0, 0));

        press_char(&mut world, 'r');
        press(&mut world, KeyCode::Esc);
        assert_eq!(sections(&world), [(1, "expired!".to_string())]);
        assert_eq!(selection(&world), Selection::Section(0, 0));
    }

    #[test]
    fn capital_d_on_divider_merges_sections_and_selects_first_event() {
        let mut world = world_with_selection(TWO_SECTIONS, Selection::Section(0, 0));
        press_char(&mut world, 'D');

        assert_eq!(sections(&world).len(), 0);
        assert_eq!(event_texts(&world), ["1", "2", "3", "4"]);
        assert_eq!(selection(&world), Selection::Event(0));
        assert_eq!(status(&world), "Section removed");

        press_char(&mut world, 'u');
        assert_eq!(sections(&world).len(), 1);
    }

    #[test]
    fn j_and_k_move_the_selected_divider() {
        let mut world = world_with_selection(TWO_SECTIONS, Selection::Section(0, 0));

        press_char(&mut world, 'J');
        assert_eq!(sections(&world)[0].0, 3);
        press_char(&mut world, 'J');
        assert_eq!(sections(&world)[0].0, 3);
        assert_eq!(selection(&world), Selection::Section(0, 0));

        press_char(&mut world, 'K');
        press_char(&mut world, 'K');
        press_char(&mut world, 'K');
        assert_eq!(sections(&world)[0].0, 1);
    }

    #[test]
    fn m_and_capital_m_on_divider_insert_below_and_above() {
        let mut world = world_with_selection(TWO_SECTIONS, Selection::Section(0, 0));
        insert_message_via_keys(&mut world, 'm', "below");
        assert_eq!(event_texts(&world), ["1", "2", "below", "3", "4"]);
        assert_eq!(sections(&world)[0].0, 2);
        assert_eq!(selection(&world), Selection::Event(2));

        let mut world = world_with_selection(TWO_SECTIONS, Selection::Section(0, 0));
        insert_message_via_keys(&mut world, 'M', "above");
        assert_eq!(event_texts(&world), ["1", "2", "above", "3", "4"]);
        assert_eq!(sections(&world)[0].0, 3);
        assert_eq!(selection(&world), Selection::Event(2));
    }

    #[test]
    fn f_and_capital_f_on_divider_act_on_the_frame() {
        let mut world = world_with_selection(TWO_SECTIONS, Selection::Section(0, 0));
        press_char(&mut world, 'f');
        assert_eq!(world.get::<EditorState>().mode, EditorMode::SelectFrameKind);
        press(&mut world, KeyCode::Esc);

        press_char(&mut world, 'F');
        assert_eq!(world.get::<SequenceDiagram>().fragments.len(), 0);
        assert_eq!(selection(&world), Selection::Event(0));
    }

    #[test]
    fn moving_an_event_across_a_divider_moves_the_divider() {
        let mut world = world_with_selection(TWO_SECTIONS, Selection::Event(1));
        press_char(&mut world, 'J');

        assert_eq!(sections(&world)[0].0, 1);
        assert_eq!(selection(&world), Selection::Event(1));
    }

    #[test]
    fn move_that_empties_a_section_reports_it() {
        let source = TWO_SECTIONS.replace("A->>B: 4\n", "");
        let mut world = world_with_selection(&source, Selection::Event(2));
        press_char(&mut world, 'K');

        assert_eq!(sections(&world).len(), 0);
        assert_eq!(status(&world), "Section removed");
    }

    #[test]
    fn changing_kind_to_one_without_dividers_clears_sections() {
        let mut world = world_with_selection(TWO_SECTIONS, Selection::Section(0, 0));
        press_char(&mut world, 'f');
        press_char(&mut world, 'j');
        press(&mut world, KeyCode::Enter);
        press(&mut world, KeyCode::Enter);

        let diagram = world.get::<SequenceDiagram>();
        assert_eq!(diagram.fragments[0].kind, FragmentKind::Opt);
        assert_eq!(diagram.fragments[0].sections.len(), 0);
        assert_eq!(status(&world), "Sections removed");
        assert_eq!(selection(&world), Selection::Fragment(0));

        press_char(&mut world, 'u');
        assert_eq!(
            world.get::<SequenceDiagram>().fragments[0].kind,
            FragmentKind::Alt
        );
        assert_eq!(sections(&world).len(), 1);
        assert_eq!(selection(&world), Selection::Section(0, 0));
    }

    #[test]
    fn changing_alt_to_par_keeps_sections_and_exports_and() {
        let mut world = world_with_selection(TWO_SECTIONS, Selection::Section(0, 0));
        press_char(&mut world, 'f');
        press_char(&mut world, 'j');
        press_char(&mut world, 'j');
        press(&mut world, KeyCode::Enter);
        press(&mut world, KeyCode::Enter);

        assert_eq!(
            world.get::<SequenceDiagram>().fragments[0].kind,
            FragmentKind::Par
        );
        assert_eq!(sections(&world).len(), 1);
        assert_eq!(selection(&world), Selection::Section(0, 0));
        assert!(
            world
                .get::<SequenceDiagram>()
                .to_mermaid()
                .contains("and b")
        );

        let screen = screen_text(&draw(&mut world, 80, 24));
        assert!(screen.contains("par [ok]"), "{screen}");
    }

    #[test]
    fn s_works_in_par_and_critical_frames() {
        for keyword in ["par", "critical"] {
            let source = THREE_EVENTS_IN_ALT.replace("alt", keyword);
            let mut world = world_with_selection(&source, Selection::Event(2));
            press_char(&mut world, 's');
            press(&mut world, KeyCode::Enter);

            assert_eq!(selection(&world), Selection::Section(0, 0), "{keyword}");
            let divider = if keyword == "par" { "and" } else { "option" };
            let exported = world.get::<SequenceDiagram>().to_mermaid();
            assert!(exported.contains(&format!("    {divider}\n")), "{exported}");
        }
    }

    #[test]
    fn restoring_a_diagram_without_the_divider_selects_its_frame() {
        let mut world = world_with_selection(TWO_SECTIONS, Selection::Section(0, 0));
        let mut diagram = world.get::<SequenceDiagram>().clone();
        diagram.fragments[0].sections.clear();

        restore_diagram(&mut world, diagram, Selection::Section(0, 0), "Undo");

        assert_eq!(selection(&world), Selection::Fragment(0));
    }
}
