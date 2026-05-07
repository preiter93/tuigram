#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum NotePosition {
    #[default]
    Right,
    Left,
    Over,
}

impl NotePosition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Right => "right of",
            Self::Left => "left of",
            Self::Over => "over",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Right => Self::Left,
            Self::Left => Self::Over,
            Self::Over => Self::Right,
        }
    }

    pub fn prev(self) -> Self {
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

impl Event {
    pub const fn height(&self) -> u16 {
        match self {
            Self::Message { .. } => 3,
            Self::Note { .. } => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BoxColor {
    #[default]
    Blue,
    Green,
    Red,
    Yellow,
    Orange,
    Purple,
    Aqua,
    Gray,
}

impl BoxColor {
    pub fn as_mermaid_str(self) -> &'static str {
        match self {
            Self::Blue => "Blue",
            Self::Green => "Green",
            Self::Red => "Red",
            Self::Yellow => "Yellow",
            Self::Orange => "Orange",
            Self::Purple => "Purple",
            Self::Aqua => "Aqua",
            Self::Gray => "Gray",
        }
    }

    pub fn from_mermaid_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "blue" => Some(Self::Blue),
            "green" => Some(Self::Green),
            "red" => Some(Self::Red),
            "yellow" => Some(Self::Yellow),
            "orange" => Some(Self::Orange),
            "purple" => Some(Self::Purple),
            "aqua" | "cyan" => Some(Self::Aqua),
            "gray" | "grey" => Some(Self::Gray),
            _ => None,
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Blue => Self::Green,
            Self::Green => Self::Red,
            Self::Red => Self::Yellow,
            Self::Yellow => Self::Orange,
            Self::Orange => Self::Purple,
            Self::Purple => Self::Aqua,
            Self::Aqua => Self::Gray,
            Self::Gray => Self::Blue,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Blue => Self::Gray,
            Self::Green => Self::Blue,
            Self::Red => Self::Green,
            Self::Yellow => Self::Red,
            Self::Orange => Self::Yellow,
            Self::Purple => Self::Orange,
            Self::Aqua => Self::Purple,
            Self::Gray => Self::Aqua,
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::Blue,
            Self::Green,
            Self::Red,
            Self::Yellow,
            Self::Orange,
            Self::Purple,
            Self::Aqua,
            Self::Gray,
        ]
    }
}

#[derive(Clone, Debug)]
pub struct ParticipantBox {
    pub label: String,
    pub color: BoxColor,
    pub start: usize,
    pub end: usize,
}
