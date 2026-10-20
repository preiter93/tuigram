use super::Selection;
use crate::core::NotePosition;
use crate::core::{BoxColor, FragmentKind};
use std::{
    ops::RangeInclusive,
    time::{Duration, Instant},
};

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
    SelectBoxEnd,
    SelectBoxColor,
    InputBoxLabel,
    SelectFrameEnd,
    SelectFrameKind,
    InputFrameLabel,
    InputSectionLabel,
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
        )
    }

    pub fn is_selecting_from(&self) -> bool {
        matches!(self, Self::SelectFrom | Self::EditSelectFrom)
    }

    pub fn is_message_text_input(&self) -> bool {
        matches!(
            self,
            Self::InputMessage | Self::EditMessage | Self::InputNoteText | Self::EditNoteText
        )
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
                | Self::InputSectionLabel
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
    pub selection_before: Selection,
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
    /// Event a new divider goes above while its label is typed.
    pub section_start: Option<usize>,
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
        self.section_start = None;
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

    /// Aborts a flow; a create flow also restores the selection from before it started.
    pub fn cancel(&mut self) {
        if self.box_start.is_some() || self.frame_start.is_some() || self.section_start.is_some() {
            self.selection = self.selection_before;
        }
        self.reset();
    }

    /// Events a new frame covers while it is being created: the cursor range while
    /// choosing its end, the chosen range during the kind and label steps.
    pub fn frame_selection_range(&self) -> Option<RangeInclusive<usize>> {
        let start = self.frame_start?;
        match (&self.mode, self.selection, self.frame_end) {
            (EditorMode::SelectFrameEnd, Selection::Event(cursor), _) => {
                Some(start.min(cursor)..=start.max(cursor))
            }
            (EditorMode::SelectFrameKind | EditorMode::InputFrameLabel, _, Some(end)) => {
                Some(start..=end)
            }
            _ => None,
        }
    }

    pub fn box_selection_range(&self) -> Option<RangeInclusive<usize>> {
        let start = self.box_start?;
        match (&self.mode, self.selection, self.box_end) {
            (EditorMode::SelectBoxEnd, Selection::Participant(cursor), _) => {
                Some(start.min(cursor)..=start.max(cursor))
            }
            (EditorMode::SelectBoxColor | EditorMode::InputBoxLabel, _, Some(end)) => {
                Some(start..=end)
            }
            _ => None,
        }
    }

    pub fn section_preview(&self) -> Option<usize> {
        self.section_start
            .filter(|_| self.mode == EditorMode::InputSectionLabel)
    }

    pub fn clear_selection(&mut self) {
        self.selection = Selection::None;
    }
}
