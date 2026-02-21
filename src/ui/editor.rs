use super::Selection;
use crate::core::NotePosition;
use std::time::Instant;

#[derive(Default, Clone, PartialEq, Eq)]
pub enum EditorMode {
    #[default]
    Normal,
    InputParticipant,
    SelectFrom,
    SelectTo,
    InputMessage,
    EditMessage,
    EditSelectFrom,
    EditSelectTo,
    RenameParticipant,
    Help,
    ConfirmClear,
    SelectNoteParticipant,
    SelectNotePosition,
    SelectNoteEndParticipant,
    InputNoteText,
    EditNoteParticipant,
    EditNotePosition,
    EditNoteEndParticipant,
    EditNoteText,
}

impl EditorMode {
    pub fn is_selecting_participant(&self) -> bool {
        matches!(
            self,
            Self::SelectFrom
                | Self::SelectTo
                | Self::EditSelectFrom
                | Self::EditSelectTo
                | Self::SelectNoteParticipant
                | Self::SelectNoteEndParticipant
                | Self::EditNoteParticipant
                | Self::EditNoteEndParticipant
        )
    }

    pub fn is_selecting_from(&self) -> bool {
        matches!(self, Self::SelectFrom | Self::EditSelectFrom)
    }

    pub fn is_text_input(&self) -> bool {
        matches!(
            self,
            Self::InputParticipant
                | Self::InputMessage
                | Self::EditMessage
                | Self::RenameParticipant
                | Self::InputNoteText
                | Self::EditNoteText
        )
    }
}

#[derive(Clone)]
pub struct StatusMessage {
    pub text: String,
    pub created_at: Instant,
}

#[derive(Default, Clone)]
pub struct EditorState {
    pub mode: EditorMode,
    pub input_buffer: String,
    pub selected_index: usize,
    pub message_from: Option<usize>,
    pub message_to: Option<usize>,
    pub status_message: Option<StatusMessage>,
    pub selection: Selection,
    pub editing_event_index: Option<usize>,
    pub last_participant_index: Option<usize>,
    pub note_position: NotePosition,
    pub note_participant_start: Option<usize>,
    pub note_participant_end: Option<usize>,
    pub insert_after_index: Option<usize>,
}

impl EditorState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.mode = EditorMode::Normal;
        self.input_buffer.clear();
        self.selected_index = 0;
        self.message_from = None;
        self.message_to = None;
        self.status_message = None;
        self.editing_event_index = None;
        self.note_position = NotePosition::default();
        self.note_participant_start = None;
        self.note_participant_end = None;
        self.insert_after_index = None;
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some(StatusMessage {
            text: msg.into(),
            created_at: Instant::now(),
        });
    }

    pub fn get_status(&self) -> Option<&str> {
        self.status_message.as_ref().and_then(|s| {
            if s.created_at.elapsed().as_millis() < 1000 {
                Some(s.text.as_str())
            } else {
                None
            }
        })
    }

    pub fn clear_selection(&mut self) {
        self.selection = Selection::None;
    }
}
