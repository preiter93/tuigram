use super::Selection;
use crate::core::NotePosition;
use crate::core::{BoxColor, FragmentKind};
use std::time::{Duration, Instant};

const STATUS_DURATION: Duration = Duration::from_millis(1000);
const ERROR_STATUS_DURATION: Duration = Duration::from_secs(3);

#[derive(Debug, Default, Clone, PartialEq, Eq)]
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
    ConfirmQuit,
    SelectNoteParticipant,
    SelectNotePosition,
    SelectNoteEndParticipant,
    InputNoteText,
    EditNoteParticipant,
    EditNotePosition,
    EditNoteEndParticipant,
    EditNoteText,
    SelectBoxStart,
    SelectBoxEnd,
    SelectBoxColor,
    InputBoxLabel,
    SelectFrameEnd,
    SelectFrameKind,
    InputFrameLabel,
    InputSavePath,
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
                | Self::SelectBoxStart
                | Self::SelectBoxEnd
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
                | Self::InputBoxLabel
                | Self::InputFrameLabel
                | Self::InputSavePath
        )
    }
}

#[derive(Clone)]
pub struct StatusMessage {
    pub text: String,
    pub is_error: bool,
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
    pub insert_anchor: Option<usize>,
    pub insert_before: bool,
    pub insert_frame: Option<usize>,
    pub box_start: Option<usize>,
    pub box_end: Option<usize>,
    pub box_color: BoxColor,
    pub frame_start: Option<usize>,
    pub frame_end: Option<usize>,
    pub frame_kind: FragmentKind,
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
        self.insert_anchor = None;
        self.insert_before = false;
        self.insert_frame = None;
        self.box_start = None;
        self.box_end = None;
        self.box_color = BoxColor::default();
        self.frame_start = None;
        self.frame_end = None;
        self.frame_kind = FragmentKind::default();
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some(StatusMessage {
            text: msg.into(),
            is_error: false,
            created_at: Instant::now(),
        });
    }

    pub fn set_error(&mut self, msg: impl Into<String>) {
        self.status_message = Some(StatusMessage {
            text: msg.into(),
            is_error: true,
            created_at: Instant::now(),
        });
    }

    pub fn get_status(&self) -> Option<&StatusMessage> {
        let status = self.status_message.as_ref()?;
        let visible_for = if status.is_error {
            ERROR_STATUS_DURATION
        } else {
            STATUS_DURATION
        };

        (status.created_at.elapsed() < visible_for).then_some(status)
    }

    pub fn clear_selection(&mut self) {
        self.selection = Selection::None;
    }
}
