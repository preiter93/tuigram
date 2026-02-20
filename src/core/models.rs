#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum NotePosition {
    #[default]
    Right,
    Left,
    Over,
}

impl NotePosition {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Right => "right of",
            Self::Left => "left of",
            Self::Over => "over",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Right => Self::Left,
            Self::Left => Self::Over,
            Self::Over => Self::Right,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            Self::Right => Self::Over,
            Self::Left => Self::Right,
            Self::Over => Self::Left,
        }
    }
}

#[derive(Clone)]
pub enum Event {
    Message {
        from: usize,
        to: usize,
        text: String,
    },
    Note {
        position: NotePosition,
        participant_start: usize,
        participant_end: usize,
        text: String,
    },
}
