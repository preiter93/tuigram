/// Color used for boxes without one of the named Mermaid colors.
pub const DEFAULT_BOX_COLOR: BoxColor = BoxColor::Gray;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ArrowStyle {
    #[default]
    SolidArrow,
    DashedArrow,
    Solid,
    Dashed,
    SolidCross,
    DashedCross,
    SolidAsync,
    DashedAsync,
}

impl ArrowStyle {
    pub fn as_mermaid_str(self) -> &'static str {
        match self {
            Self::SolidArrow => "->>",
            Self::DashedArrow => "-->>",
            Self::Solid => "->",
            Self::Dashed => "-->",
            Self::SolidCross => "-x",
            Self::DashedCross => "--x",
            Self::SolidAsync => "-)",
            Self::DashedAsync => "--)",
        }
    }

    pub fn from_mermaid_str(s: &str) -> Option<Self> {
        match s {
            "->>" => Some(Self::SolidArrow),
            "-->>" => Some(Self::DashedArrow),
            "->" => Some(Self::Solid),
            "-->" => Some(Self::Dashed),
            "-x" => Some(Self::SolidCross),
            "--x" => Some(Self::DashedCross),
            "-)" => Some(Self::SolidAsync),
            "--)" => Some(Self::DashedAsync),
            _ => None,
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::SolidArrow => Self::DashedArrow,
            Self::DashedArrow => Self::Solid,
            Self::Solid => Self::Dashed,
            Self::Dashed => Self::SolidCross,
            Self::SolidCross => Self::DashedCross,
            Self::DashedCross => Self::SolidAsync,
            Self::SolidAsync => Self::DashedAsync,
            Self::DashedAsync => Self::SolidArrow,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::SolidArrow => Self::DashedAsync,
            Self::DashedArrow => Self::SolidArrow,
            Self::Solid => Self::DashedArrow,
            Self::Dashed => Self::Solid,
            Self::SolidCross => Self::Dashed,
            Self::DashedCross => Self::SolidCross,
            Self::SolidAsync => Self::DashedCross,
            Self::DashedAsync => Self::SolidAsync,
        }
    }

    pub fn is_dashed(self) -> bool {
        matches!(
            self,
            Self::DashedArrow | Self::Dashed | Self::DashedCross | Self::DashedAsync
        )
    }
}

#[derive(Clone, PartialEq)]
pub enum Event {
    Message {
        from: usize,
        to: usize,
        text: String,
        style: ArrowStyle,
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

    pub const fn endpoints(&self) -> (usize, usize) {
        match self {
            Self::Message { from, to, .. } => (*from, *to),
            Self::Note {
                participant_start,
                participant_end,
                ..
            } => (*participant_start, *participant_end),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FragmentKind {
    #[default]
    Loop,
    Opt,
    Break,
    Critical,
}

impl FragmentKind {
    pub const ALL: [Self; 4] = [Self::Loop, Self::Opt, Self::Break, Self::Critical];

    pub fn as_mermaid_str(self) -> &'static str {
        match self {
            Self::Loop => "loop",
            Self::Opt => "opt",
            Self::Break => "break",
            Self::Critical => "critical",
        }
    }

    pub fn from_mermaid_str(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.as_mermaid_str() == s)
    }

    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|k| *k == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Self {
        let idx = Self::ALL.iter().position(|k| *k == self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }

    pub fn split_header(line: &str) -> Option<(Self, String)> {
        let line = line.trim();
        let (word, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        let kind = Self::from_mermaid_str(word)?;
        Some((kind, rest.trim().to_string()))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Fragment {
    pub kind: FragmentKind,
    pub label: String,
    pub start: usize,
    pub end: usize,
}

impl Fragment {
    pub fn header(&self) -> String {
        let kind = self.kind.as_mermaid_str();
        if self.label.is_empty() {
            kind.to_string()
        } else {
            format!("{kind} {}", self.label)
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

#[derive(Clone, Debug, PartialEq)]
pub struct ParticipantBox {
    pub label: String,
    pub color: Option<BoxColor>,
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Participant {
    pub id: String,
    pub label: Option<String>,
    pub actor: bool,
}

impl Participant {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: None,
            actor: false,
        }
    }

    pub fn display_name(&self) -> &str {
        self.label.as_deref().unwrap_or(&self.id)
    }
}
