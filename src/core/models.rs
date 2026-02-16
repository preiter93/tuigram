pub struct Participant(pub String);

#[derive(Clone)]
pub enum Event {
    Message {
        from: usize,
        to: usize,
        text: String,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    Sync,
    Async,
}

#[derive(Default, Clone, PartialEq, Eq)]
pub enum EditorMode {
    #[default]
    Normal,
    InputParticipant,
    SelectFrom,
    SelectTo,
    InputMessage,
    Help,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    #[default]
    None,
    Participant(usize),
    Event(usize),
}

#[derive(Default, Clone)]
pub struct EditorState {
    pub mode: EditorMode,
    pub input_buffer: String,
    pub selected_index: usize,
    pub message_from: Option<usize>,
    pub message_to: Option<usize>,
    pub status_message: Option<String>,
    pub selection: Selection,
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
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some(msg.into());
    }

    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    pub fn clear_selection(&mut self) {
        self.selection = Selection::None;
    }

    pub fn select_participant(&mut self, index: usize) {
        self.selection = Selection::Participant(index);
    }

    pub fn select_event(&mut self, index: usize) {
        self.selection = Selection::Event(index);
    }

    pub fn selected_participant(&self) -> Option<usize> {
        match self.selection {
            Selection::Participant(i) => Some(i),
            _ => None,
        }
    }

    pub fn selected_event(&self) -> Option<usize> {
        match self.selection {
            Selection::Event(i) => Some(i),
            _ => None,
        }
    }
}
