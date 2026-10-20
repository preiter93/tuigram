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

/// Lines are trimmed; the result always has at least one entry.
pub fn text_lines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut line_start = 0;
    let mut pos = 0;

    while let Some(offset) = text[pos..].find('<') {
        let tag_start = pos + offset;
        match line_break_len(&text[tag_start..]) {
            Some(len) => {
                lines.push(text[line_start..tag_start].trim());
                pos = tag_start + len;
                line_start = pos;
            }
            None => pos = tag_start + 1,
        }
    }

    lines.push(text[line_start..].trim());
    lines
}

fn line_break_len(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    if bytes.len() < 3 || !bytes[..3].eq_ignore_ascii_case(b"<br") {
        return None;
    }

    let mut len = 3;
    while bytes.get(len).is_some_and(u8::is_ascii_whitespace) {
        len += 1;
    }
    if bytes.get(len) == Some(&b'/') {
        len += 1;
    }

    (bytes.get(len) == Some(&b'>')).then_some(len + 1)
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
    pub fn text(&self) -> &str {
        match self {
            Self::Message { text, .. } | Self::Note { text, .. } => text,
        }
    }

    pub fn height(&self) -> u16 {
        let lines = u16::try_from(text_lines(self.text()).len()).unwrap_or(u16::MAX);
        match self {
            Self::Message { .. } => lines.saturating_add(2),
            Self::Note { .. } => lines.saturating_add(1),
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
    Alt,
    Opt,
    Par,
    Break,
    Critical,
}

impl FragmentKind {
    pub const ALL: [Self; 6] = [
        Self::Loop,
        Self::Alt,
        Self::Opt,
        Self::Par,
        Self::Critical,
        Self::Break,
    ];

    pub fn as_mermaid_str(self) -> &'static str {
        match self {
            Self::Loop => "loop",
            Self::Alt => "alt",
            Self::Opt => "opt",
            Self::Par => "par",
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

    /// Keyword of the line that starts a new section inside this kind of frame.
    pub fn divider(self) -> Option<&'static str> {
        match self {
            Self::Alt => Some("else"),
            Self::Par => Some("and"),
            Self::Critical => Some("option"),
            Self::Loop | Self::Opt | Self::Break => None,
        }
    }

    /// The kind whose sections are separated by `word`.
    pub fn with_divider(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.divider() == Some(word))
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
    /// Dividers as (index of the first event below, label). Starts increase
    /// strictly and each lies in `(start, end]`.
    pub sections: Vec<(usize, String)>,
}

impl Fragment {
    pub fn header(&self) -> String {
        keyword_line(self.kind.as_mermaid_str(), &self.label)
    }

    /// Keyword and label of the divider line at `sections[k]`.
    pub fn divider_source(&self, k: usize) -> Option<String> {
        let word = self.kind.divider()?;
        let (_, label) = self.sections.get(k)?;
        Some(keyword_line(word, label))
    }

    pub fn move_to(&mut self, start: usize) {
        let old_start = self.start;
        self.end = self.end - old_start + start;
        self.start = start;
        for (at, _) in &mut self.sections {
            *at = *at - old_start + start;
        }
    }

    /// Restores the section invariants after events were removed or moved.
    pub fn drop_empty_sections(&mut self) {
        let (start, end) = (self.start, self.end);

        while self.sections.first().is_some_and(|(at, _)| *at <= start) {
            let (_, label) = self.sections.remove(0);
            self.label = label;
        }
        self.sections.retain(|(at, _)| *at <= end);

        let mut k = 1;
        while k < self.sections.len() {
            if self.sections[k - 1].0 >= self.sections[k].0 {
                self.sections.remove(k - 1);
            } else {
                k += 1;
            }
        }
    }
}

fn keyword_line(keyword: &str, label: &str) -> String {
    if label.is_empty() {
        keyword.to_string()
    } else {
        format!("{keyword} {label}")
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

#[cfg(test)]
mod tests {
    use super::*;

    fn message(text: &str) -> Event {
        Event::Message {
            from: 0,
            to: 1,
            text: text.to_string(),
            style: ArrowStyle::default(),
        }
    }

    fn note(text: &str) -> Event {
        Event::Note {
            position: NotePosition::Right,
            participant_start: 0,
            participant_end: 0,
            text: text.to_string(),
        }
    }

    #[test]
    fn text_lines_split_on_break_tags() {
        for text in ["a<br>b", "a<br/>b", "a<BR />b", "a <br/> b"] {
            assert_eq!(text_lines(text), ["a", "b"], "{text}");
        }
    }

    #[test]
    fn text_lines_always_has_one_entry() {
        assert_eq!(text_lines("x"), ["x"]);
        assert_eq!(text_lines(""), [""]);
    }

    #[test]
    fn text_lines_keep_other_tags_literal() {
        assert_eq!(text_lines("a<bra>"), ["a<bra>"]);
        assert_eq!(text_lines("1 < 2 <br> é"), ["1 < 2", "é"]);
    }

    #[test]
    fn event_height_grows_with_lines() {
        assert_eq!(message("a").height(), 3);
        assert_eq!(message("a<br/>b").height(), 4);
        assert_eq!(message("a<br/>b<br>c").height(), 5);
        assert_eq!(note("a").height(), 2);
        assert_eq!(note("a<br/>b").height(), 3);
        assert_eq!(note("a<br/>b<br/>c").height(), 4);
    }
}
