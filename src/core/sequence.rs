use super::models::{ArrowStyle, BoxColor, Event, NotePosition, Participant, ParticipantBox};
use anyhow::{Result, anyhow, bail};

const BLOCK_KEYWORDS: &[&str] = &[
    "alt", "else", "loop", "opt", "par", "and", "critical", "option", "break", "rect",
];
const OTHER_KEYWORDS: &[&str] = &["activate", "deactivate", "autonumber", "create", "destroy"];
const RESERVED_WORDS: &[&str] = &[
    "end",
    "box",
    "participant",
    "actor",
    "note",
    "sequenceDiagram",
];
const ARROWS: &[&str] = &["-->>", "->>", "--x", "-x", "--)", "-)", "-->", "->"];

#[derive(Default, Clone, PartialEq)]
pub struct SequenceDiagram {
    pub participants: Vec<Participant>,
    pub events: Vec<Event>,
    pub boxes: Vec<ParticipantBox>,
}

impl SequenceDiagram {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_box(&mut self, label: String, color: BoxColor, start: usize, end: usize) -> bool {
        if start >= self.participants.len() || end >= self.participants.len() || start > end {
            return false;
        }
        // Reject if this range overlaps any existing box.
        let overlaps = self.boxes.iter().any(|b| start <= b.end && b.start <= end);
        if overlaps {
            return false;
        }
        self.boxes.push(ParticipantBox {
            label,
            color: Some(color),
            start,
            end,
        });
        self.boxes.sort_by_key(|b| b.start);
        true
    }

    pub fn remove_box_at(&mut self, idx: usize) {
        if idx < self.boxes.len() {
            self.boxes.remove(idx);
        }
    }

    #[allow(dead_code)]
    pub fn box_for_participant(&self, participant_idx: usize) -> Option<&ParticipantBox> {
        self.boxes
            .iter()
            .find(|b| b.start <= participant_idx && participant_idx <= b.end)
    }

    pub fn add_participant(&mut self, name: &str) {
        if is_valid_id(name) && self.index_of(name).is_none() {
            self.participants.push(Participant::new(name));
            return;
        }

        let mut participant = Participant::new(self.make_id(name));
        participant.label = Some(name.to_string());
        self.participants.push(participant);
    }

    fn index_of(&self, id: &str) -> Option<usize> {
        self.participants.iter().position(|p| p.id == id)
    }

    fn make_id(&self, name: &str) -> String {
        let valid: String = name
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        let base = if valid.is_empty() {
            "P".to_string()
        } else {
            valid
        };

        let mut id = base.clone();
        let mut n = 2;
        while self.index_of(&id).is_some() || is_reserved(&id) {
            id = format!("{base}{n}");
            n += 1;
        }
        id
    }

    pub fn rename_participant(&mut self, idx: usize, name: &str) {
        let Some(participant) = self.participants.get(idx) else {
            return;
        };

        if participant.label.is_some() {
            let label = (name != participant.id).then(|| name.to_string());
            self.participants[idx].label = label;
            return;
        }

        if is_valid_id(name) && self.index_of(name).is_none() {
            self.participants[idx].id = name.to_string();
        } else if name != participant.id {
            self.participants[idx].label = Some(name.to_string());
        }
    }

    pub fn toggle_actor(&mut self, idx: usize) -> Option<bool> {
        let participant = self.participants.get_mut(idx)?;
        participant.actor = !participant.actor;
        Some(participant.actor)
    }

    pub fn add_message(&mut self, from: usize, to: usize, text: String) {
        if from < self.participants.len() && to < self.participants.len() {
            self.events.push(Event::Message {
                from,
                to,
                text,
                style: ArrowStyle::default(),
            });
        }
    }

    pub fn insert_message(&mut self, after_index: usize, from: usize, to: usize, text: String) {
        if from < self.participants.len() && to < self.participants.len() {
            let insert_at = (after_index + 1).min(self.events.len());
            self.events.insert(
                insert_at,
                Event::Message {
                    from,
                    to,
                    text,
                    style: ArrowStyle::default(),
                },
            );
        }
    }

    pub fn add_note(
        &mut self,
        position: NotePosition,
        participant_start: usize,
        participant_end: usize,
        text: String,
    ) {
        if participant_start < self.participants.len() && participant_end < self.participants.len()
        {
            self.events.push(Event::Note {
                position,
                participant_start,
                participant_end,
                text,
            });
        }
    }

    pub fn insert_note(
        &mut self,
        after_index: usize,
        position: NotePosition,
        participant_start: usize,
        participant_end: usize,
        text: String,
    ) {
        if participant_start < self.participants.len() && participant_end < self.participants.len()
        {
            let insert_at = (after_index + 1).min(self.events.len());
            self.events.insert(
                insert_at,
                Event::Note {
                    position,
                    participant_start,
                    participant_end,
                    text,
                },
            );
        }
    }

    pub fn participant_count(&self) -> usize {
        self.participants.len()
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    pub fn remove_event(&mut self, idx: usize) {
        if idx < self.events.len() {
            self.events.remove(idx);
        }
    }

    pub fn swap_participants(&mut self, a: usize, b: usize) {
        if a >= self.participants.len() || b >= self.participants.len() {
            return;
        }
        self.participants.swap(a, b);
        for e in &mut self.events {
            match e {
                Event::Message { from, to, .. } => {
                    if *from == a {
                        *from = b;
                    } else if *from == b {
                        *from = a;
                    }
                    if *to == a {
                        *to = b;
                    } else if *to == b {
                        *to = a;
                    }
                }
                Event::Note {
                    participant_start,
                    participant_end,
                    ..
                } => {
                    if *participant_start == a {
                        *participant_start = b;
                    } else if *participant_start == b {
                        *participant_start = a;
                    }
                    if *participant_end == a {
                        *participant_end = b;
                    } else if *participant_end == b {
                        *participant_end = a;
                    }
                }
            }
        }
    }

    pub fn point_event_left(&mut self, idx: usize) {
        if let Some(Event::Message { from, to, .. }) = self.events.get_mut(idx)
            && *from < *to
        {
            std::mem::swap(from, to);
        }
    }

    pub fn point_event_right(&mut self, idx: usize) {
        if let Some(Event::Message { from, to, .. }) = self.events.get_mut(idx)
            && *from > *to
        {
            std::mem::swap(from, to);
        }
    }

    pub fn next_arrow_style(&mut self, idx: usize) -> Option<ArrowStyle> {
        if let Some(Event::Message { style, .. }) = self.events.get_mut(idx) {
            *style = style.next();
            return Some(*style);
        }
        None
    }

    pub fn prev_arrow_style(&mut self, idx: usize) -> Option<ArrowStyle> {
        if let Some(Event::Message { style, .. }) = self.events.get_mut(idx) {
            *style = style.prev();
            return Some(*style);
        }
        None
    }

    pub fn remove_participant(&mut self, idx: usize) {
        if idx >= self.participants.len() {
            return;
        }
        self.participants.remove(idx);
        self.events.retain(|e| match e {
            Event::Message { from, to, .. } => *from != idx && *to != idx,
            Event::Note {
                participant_start,
                participant_end,
                ..
            } => *participant_start != idx && *participant_end != idx,
        });
        for e in &mut self.events {
            match e {
                Event::Message { from, to, .. } => {
                    if *from > idx {
                        *from -= 1;
                    }
                    if *to > idx {
                        *to -= 1;
                    }
                }
                Event::Note {
                    participant_start,
                    participant_end,
                    ..
                } => {
                    if *participant_start > idx {
                        *participant_start -= 1;
                    }
                    if *participant_end > idx {
                        *participant_end -= 1;
                    }
                }
            }
        }

        let mut new_boxes: Vec<ParticipantBox> = Vec::new();
        for mut b in self.boxes.drain(..) {
            if b.start == idx && b.end == idx {
                continue;
            }
            if b.end >= idx {
                b.end = b.end.saturating_sub(1);
            }
            if b.start > idx {
                b.start -= 1;
            }
            if b.start <= b.end {
                new_boxes.push(b);
            }
        }

        self.boxes = new_boxes;
    }

    pub fn to_mermaid(&self) -> String {
        let mut lines = vec!["sequenceDiagram".to_string()];

        let mut i = 0;
        while i < self.participants.len() {
            if let Some(b) = self.boxes.iter().find(|b| b.start == i) {
                let mut header = String::from("    box");
                if let Some(color) = b.color {
                    header.push(' ');
                    header.push_str(color.as_mermaid_str());
                }
                if !b.label.is_empty() {
                    header.push(' ');
                    header.push_str(&b.label);
                }
                lines.push(header);
                let box_end = b.end;
                for j in i..=box_end {
                    if let Some(p) = self.participants.get(j) {
                        lines.push(format!("        {}", declaration(p)));
                    }
                }
                lines.push("    end".to_string());
                i = box_end + 1;
            } else {
                if let Some(p) = self.participants.get(i) {
                    lines.push(format!("    {}", declaration(p)));
                }
                i += 1;
            }
        }

        for event in &self.events {
            match event {
                Event::Message {
                    from,
                    to,
                    text,
                    style,
                } => {
                    if let (Some(from_p), Some(to_p)) =
                        (self.participants.get(*from), self.participants.get(*to))
                    {
                        let (from_name, to_name) = (&from_p.id, &to_p.id);
                        let arrow = style.as_mermaid_str();
                        lines.push(format!("    {from_name}{arrow}{to_name}: {text}"));
                    }
                }
                Event::Note {
                    position,
                    participant_start,
                    participant_end,
                    text,
                } => {
                    let pos_str = position.as_str();
                    if *position == NotePosition::Over && participant_start != participant_end {
                        if let (Some(start_p), Some(end_p)) = (
                            self.participants.get(*participant_start),
                            self.participants.get(*participant_end),
                        ) {
                            let (start_name, end_name) = (&start_p.id, &end_p.id);
                            lines.push(format!(
                                "    Note {pos_str} {start_name},{end_name}: {text}"
                            ));
                        }
                    } else if let Some(p) = self.participants.get(*participant_start) {
                        let name = &p.id;
                        lines.push(format!("    Note {pos_str} {name}: {text}"));
                    }
                }
            }
        }

        lines.join("\n") + "\n"
    }

    pub fn from_mermaid(input: &str) -> Result<Self> {
        let mut diagram = SequenceDiagram::new();
        let mut current_box: Option<(Option<BoxColor>, String, usize)> = None;
        let mut seen_header = false;

        for (idx, line) in input.lines().enumerate() {
            let n = idx + 1;
            let trimmed = line.trim();

            if trimmed.is_empty() || trimmed.starts_with("%%") {
                continue;
            }

            if !seen_header {
                if trimmed != "sequenceDiagram" {
                    bail!("line {n}: first line must be 'sequenceDiagram'");
                }
                seen_header = true;
                continue;
            }

            diagram
                .parse_line(&mut current_box, trimmed)
                .map_err(|e| anyhow!("line {n}: {e}"))?;
        }

        if !seen_header {
            bail!("missing 'sequenceDiagram' header");
        }
        if current_box.is_some() {
            bail!("'box' is never closed");
        }

        Ok(diagram)
    }

    fn parse_line(
        &mut self,
        current_box: &mut Option<(Option<BoxColor>, String, usize)>,
        trimmed: &str,
    ) -> Result<()> {
        if let Some(first_word) = trimmed.split_whitespace().next() {
            if BLOCK_KEYWORDS.contains(&first_word) {
                bail!("'{first_word}' blocks are not supported yet");
            }
            if OTHER_KEYWORDS.contains(&first_word) {
                bail!("'{first_word}' is not supported yet");
            }
        }

        if let Some(rest) = trimmed.strip_prefix("participant ") {
            return self.parse_declaration(rest, false, current_box.is_some(), trimmed);
        }

        if let Some(rest) = trimmed.strip_prefix("actor ") {
            return self.parse_declaration(rest, true, current_box.is_some(), trimmed);
        }

        if let Some(rest) = trimmed.strip_prefix("Note ") {
            let (position, after_pos) = if let Some(after) = rest.strip_prefix("right of ") {
                (NotePosition::Right, after)
            } else if let Some(after) = rest.strip_prefix("left of ") {
                (NotePosition::Left, after)
            } else if let Some(after) = rest.strip_prefix("over ") {
                (NotePosition::Over, after)
            } else {
                bail!("invalid note position: {trimmed}");
            };

            let Some(colon_pos) = after_pos.find(':') else {
                bail!("invalid note syntax (missing ':'): {trimmed}");
            };

            let participants_str = after_pos[..colon_pos].trim();
            let text = after_pos[colon_pos + 1..].trim().to_string();

            if position == NotePosition::Over && participants_str.contains(',') {
                let parts: Vec<&str> = participants_str.split(',').map(str::trim).collect();
                if parts.len() != 2 {
                    bail!("note over must have exactly 2 participants: {trimmed}");
                }
                let start_name = parts[0];
                let end_name = parts[1];

                let start_idx = self.index_of_or_insert(start_name)?;
                let end_idx = self.index_of_or_insert(end_name)?;

                self.events.push(Event::Note {
                    position,
                    participant_start: start_idx,
                    participant_end: end_idx,
                    text,
                });
            } else {
                let name = participants_str;
                if name.is_empty() {
                    bail!("invalid note syntax: {trimmed}");
                }

                let idx = self.index_of_or_insert(name)?;

                self.events.push(Event::Note {
                    position,
                    participant_start: idx,
                    participant_end: idx,
                    text,
                });
            }
            return Ok(());
        }

        if let Some((arrow_pos, style)) = find_arrow(trimmed) {
            let from_name = trimmed[..arrow_pos].trim();
            let rest = &trimmed[arrow_pos + style.as_mermaid_str().len()..];

            let (to_name, message) = if let Some(colon_pos) = rest.find(':') {
                let to = rest[..colon_pos].trim();
                let msg = rest[colon_pos + 1..].trim();
                (to, msg.to_string())
            } else {
                bail!("invalid message syntax (missing ':'): {trimmed}");
            };

            if to_name.starts_with(['+', '-']) {
                bail!("activation shorthand ('+'/'-') is not supported yet");
            }

            if from_name.is_empty() || to_name.is_empty() {
                bail!("invalid message syntax: {trimmed}");
            }

            if from_name.contains(['<', '>']) || to_name.contains(['<', '>']) {
                bail!("bidirectional arrows ('<<->>') are not supported yet");
            }

            let from_idx = self.index_of_or_insert(from_name)?;
            let to_idx = self.index_of_or_insert(to_name)?;

            self.events.push(Event::Message {
                from: from_idx,
                to: to_idx,
                text: message,
                style,
            });
            return Ok(());
        }

        if trimmed == "box" || trimmed.starts_with("box ") {
            if current_box.is_some() {
                bail!("nested boxes are not supported");
            }
            let rest = trimmed.strip_prefix("box").unwrap().trim();
            let (color, label) = if rest.is_empty() {
                (None, String::new())
            } else {
                let (first_word, remainder) = rest
                    .split_once(|c: char| c.is_whitespace())
                    .map_or((rest, ""), |(f, r)| (f, r.trim()));
                if let Some(c) = BoxColor::from_mermaid_str(first_word) {
                    (Some(c), remainder.to_string())
                } else {
                    (None, rest.to_string())
                }
            };
            *current_box = Some((color, label, self.participants.len()));
            return Ok(());
        }

        if trimmed == "end" {
            if let Some((color, label, start)) = current_box.take() {
                let end = self.participants.len().saturating_sub(1);
                if end < start {
                    bail!("box has no participants");
                }
                self.boxes.push(ParticipantBox {
                    label,
                    color,
                    start,
                    end,
                });
            } else {
                bail!("unexpected 'end'");
            }
            return Ok(());
        }

        bail!("unsupported mermaid feature: {trimmed}");
    }

    fn index_of_or_insert(&mut self, id: &str) -> Result<usize> {
        if let Some(idx) = self.index_of(id) {
            return Ok(idx);
        }
        check_id(id)?;
        self.participants.push(Participant::new(id));
        Ok(self.participants.len() - 1)
    }

    fn parse_declaration(
        &mut self,
        rest: &str,
        actor: bool,
        in_box: bool,
        line: &str,
    ) -> Result<()> {
        if rest.trim() == "as" || rest.trim_end().ends_with(" as") {
            bail!("invalid participant declaration: {line}");
        }

        let (id, label) = match rest.split_once(" as ") {
            Some((id, label)) => (id.trim(), Some(label.trim())),
            None => (rest.trim(), None),
        };
        if id.is_empty() {
            bail!("invalid participant declaration: {line}");
        }
        if in_box && self.index_of(id).is_some() {
            bail!("box participants must be declared before use");
        }

        let idx = self.index_of_or_insert(id)?;
        let participant = &mut self.participants[idx];
        participant.actor = actor;
        if let Some(label) = label {
            participant.label = Some(label.to_string());
        }
        Ok(())
    }
}

fn declaration(p: &Participant) -> String {
    let keyword = if p.actor { "actor" } else { "participant" };
    match &p.label {
        Some(label) => format!("{keyword} {} as {label}", p.id),
        None => format!("{keyword} {}", p.id),
    }
}

fn is_valid_id(s: &str) -> bool {
    !s.is_empty() && !is_reserved(s) && s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

fn is_reserved(s: &str) -> bool {
    BLOCK_KEYWORDS
        .iter()
        .chain(OTHER_KEYWORDS)
        .chain(RESERVED_WORDS)
        .any(|word| word.eq_ignore_ascii_case(s))
}

fn check_id(id: &str) -> Result<()> {
    let has_special = id.contains([',', ':', ';', '<', '>']);
    let starts_special = id.starts_with(['+', '-']) || id.starts_with("%%");
    let has_as = id.split_whitespace().any(|w| w == "as");
    let ends_dash = id.ends_with('-');
    let starts_keyword = id
        .split_once(char::is_whitespace)
        .is_some_and(|(first, _)| is_reserved(first));

    let invalid = has_special || starts_special || has_as || ends_dash || starts_keyword;
    if invalid || find_arrow(id).is_some() {
        bail!("unsupported participant id: {id}");
    }
    Ok(())
}

fn find_arrow(s: &str) -> Option<(usize, ArrowStyle)> {
    let before_colon = s.split(':').next().unwrap_or(s);

    if let Some(pos) = before_colon.find("->>")
        && !before_colon[..pos].ends_with('-')
    {
        return Some((pos, ArrowStyle::SolidArrow));
    }

    for (pos, _) in before_colon.match_indices('-') {
        for arrow in ARROWS {
            if before_colon[pos..].starts_with(arrow) {
                let style = ArrowStyle::from_mermaid_str(arrow).expect("ARROWS holds valid tokens");
                return Some((pos, style));
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(d: &SequenceDiagram) -> Vec<&str> {
        d.participants.iter().map(|p| p.id.as_str()).collect()
    }

    #[test]
    fn test_box_roundtrip() {
        let mut diagram = SequenceDiagram::new();
        diagram.add_participant("Alice");
        diagram.add_participant("Bob");
        diagram.add_participant("Carol");
        diagram.add_box("Frontend".to_string(), BoxColor::Blue, 0, 1);
        diagram.add_message(0, 1, "Hello".to_string());

        let mermaid = diagram.to_mermaid();
        let parsed = SequenceDiagram::from_mermaid(&mermaid).unwrap();

        assert_eq!(parsed.participants, diagram.participants);
        assert_eq!(parsed.boxes.len(), 1);
        assert_eq!(parsed.boxes[0].label, "Frontend");
        assert_eq!(parsed.boxes[0].color, Some(BoxColor::Blue));
        assert_eq!(parsed.boxes[0].start, 0);
        assert_eq!(parsed.boxes[0].end, 1);
    }

    #[test]
    fn test_box_remove_participant() {
        let mut diagram = SequenceDiagram::new();
        diagram.add_participant("Alice");
        diagram.add_participant("Bob");
        diagram.add_participant("Carol");
        diagram.add_box("Group".to_string(), BoxColor::Green, 0, 2);

        diagram.remove_participant(0); // remove Alice
        assert_eq!(diagram.boxes.len(), 1);
        assert_eq!(diagram.boxes[0].start, 0);
        assert_eq!(diagram.boxes[0].end, 1);

        diagram.remove_participant(1); // remove Carol (now at index 1)
        assert_eq!(diagram.boxes.len(), 1);
        assert_eq!(diagram.boxes[0].start, 0);
        assert_eq!(diagram.boxes[0].end, 0);
    }

    #[test]
    fn test_box_no_label() {
        let mut diagram = SequenceDiagram::new();
        diagram.add_participant("Alice");
        diagram.add_participant("Bob");
        diagram.add_box(String::new(), BoxColor::Red, 0, 1);

        let mermaid = diagram.to_mermaid();
        let parsed = SequenceDiagram::from_mermaid(&mermaid).unwrap();

        assert_eq!(parsed.boxes.len(), 1);
        assert_eq!(parsed.boxes[0].color, Some(BoxColor::Red));
        assert_eq!(parsed.boxes[0].label, "");
    }

    fn box_source(header: &str) -> String {
        format!("sequenceDiagram\n    box {header}\n        participant A\n    end\n")
    }

    #[test]
    fn test_box_without_color_keeps_label_and_exports_it() {
        let diagram = SequenceDiagram::from_mermaid(&box_source("Foo")).unwrap();

        assert_eq!(diagram.boxes[0].color, None);
        assert_eq!(diagram.boxes[0].label, "Foo");
        assert!(diagram.to_mermaid().contains("    box Foo\n"));
    }

    #[test]
    fn test_bare_box_roundtrips() {
        let diagram = SequenceDiagram::from_mermaid(&box_source("")).unwrap();

        assert_eq!(diagram.boxes[0].color, None);
        assert_eq!(diagram.boxes[0].label, "");

        let mermaid = diagram.to_mermaid();
        assert!(mermaid.contains("    box\n"));
        let reparsed = SequenceDiagram::from_mermaid(&mermaid).unwrap();
        assert_eq!(reparsed.to_mermaid(), mermaid);
    }

    #[test]
    fn test_box_rgb_color_is_kept_in_label() {
        let diagram = SequenceDiagram::from_mermaid(&box_source("rgb(10, 20, 30) Foo")).unwrap();

        assert_eq!(diagram.boxes[0].color, None);
        assert_eq!(diagram.boxes[0].label, "rgb(10, 20, 30) Foo");
        assert!(
            diagram
                .to_mermaid()
                .contains("    box rgb(10, 20, 30) Foo\n")
        );
    }

    #[test]
    fn test_box_color_only_has_empty_label() {
        let diagram = SequenceDiagram::from_mermaid(&box_source("Red")).unwrap();

        assert_eq!(diagram.boxes[0].color, Some(BoxColor::Red));
        assert_eq!(diagram.boxes[0].label, "");
    }

    #[test]
    fn test_box_remove_single_participant() {
        let mut diagram = SequenceDiagram::new();
        diagram.add_participant("Alice");
        diagram.add_box("Solo".to_string(), BoxColor::Purple, 0, 0);
        assert_eq!(diagram.boxes.len(), 1);

        diagram.remove_participant(0);
        assert_eq!(
            diagram.boxes.len(),
            0,
            "single-participant box should be dropped"
        );
    }

    #[test]
    fn test_box_for_participant() {
        let mut diagram = SequenceDiagram::new();
        diagram.add_participant("Alice");
        diagram.add_participant("Bob");
        diagram.add_participant("Carol");
        diagram.add_box("FE".to_string(), BoxColor::Blue, 0, 1);

        assert!(diagram.box_for_participant(0).is_some());
        assert!(diagram.box_for_participant(1).is_some());
        assert!(diagram.box_for_participant(2).is_none());
    }

    #[test]
    fn test_roundtrip() {
        let mut diagram = SequenceDiagram::new();
        diagram.add_participant("Alice");
        diagram.add_participant("Bob");
        diagram.add_message(0, 1, "Hello".to_string());
        diagram.add_message(1, 0, "Hi there".to_string());

        let mermaid = diagram.to_mermaid();
        let parsed = SequenceDiagram::from_mermaid(&mermaid).unwrap();

        assert_eq!(parsed.participants, diagram.participants);
        assert_eq!(parsed.events.len(), diagram.events.len());
    }

    fn parse_err(input: &str) -> String {
        SequenceDiagram::from_mermaid(input)
            .err()
            .expect("expected parse error")
            .to_string()
    }

    #[test]
    fn test_skips_blank_and_comment_lines() {
        let clean = "sequenceDiagram\nparticipant Alice\nAlice->>Bob: Hi";
        let noisy =
            "%% intro\n\nsequenceDiagram\n\n%% note\n  participant Alice\nAlice->>Bob: Hi\n";
        let a = SequenceDiagram::from_mermaid(clean).unwrap();
        let b = SequenceDiagram::from_mermaid(noisy).unwrap();
        assert_eq!(a.to_mermaid(), b.to_mermaid());
    }

    const ALL_STYLES: [(&str, ArrowStyle); 8] = [
        ("->>", ArrowStyle::SolidArrow),
        ("-->>", ArrowStyle::DashedArrow),
        ("->", ArrowStyle::Solid),
        ("-->", ArrowStyle::Dashed),
        ("-x", ArrowStyle::SolidCross),
        ("--x", ArrowStyle::DashedCross),
        ("-)", ArrowStyle::SolidAsync),
        ("--)", ArrowStyle::DashedAsync),
    ];

    fn first_style(d: &SequenceDiagram) -> ArrowStyle {
        match &d.events[0] {
            Event::Message { style, .. } => *style,
            Event::Note { .. } => panic!("Expected Message event"),
        }
    }

    #[test]
    fn test_parse_all_arrow_styles() {
        for (token, style) in ALL_STYLES {
            let d =
                SequenceDiagram::from_mermaid(&format!("sequenceDiagram\nA{token}B: Hi")).unwrap();
            match &d.events[0] {
                Event::Message {
                    from,
                    to,
                    style: parsed,
                    ..
                } => {
                    assert_eq!((*from, *to), (0, 1), "{token}");
                    assert_eq!(*parsed, style, "{token}");
                }
                Event::Note { .. } => panic!("Expected Message event"),
            }
        }
    }

    #[test]
    fn test_export_all_arrow_styles() {
        for (token, style) in ALL_STYLES {
            let mut d = SequenceDiagram::new();
            d.add_participant("A");
            d.add_participant("B");
            d.add_message(0, 1, "Hi".to_string());
            if let Event::Message { style: s, .. } = &mut d.events[0] {
                *s = style;
            }
            assert!(
                d.to_mermaid().contains(&format!("A{token}B: Hi")),
                "{token}"
            );
        }
    }

    #[test]
    fn test_roundtrip_all_arrow_styles() {
        for (token, _) in ALL_STYLES {
            let input =
                format!("sequenceDiagram\n    participant A\n    participant B\n    A{token}B: Hi");
            let d = SequenceDiagram::from_mermaid(&input).unwrap();
            assert_eq!(d.to_mermaid().trim_end(), input, "{token}");
        }
    }

    #[test]
    fn test_arrow_style_cycle() {
        let mut d = SequenceDiagram::new();
        d.add_participant("A");
        d.add_participant("B");
        d.add_message(0, 1, "Hi".to_string());

        let start = first_style(&d);
        for _ in 0..8 {
            d.next_arrow_style(0);
        }
        assert_eq!(first_style(&d), start);

        d.next_arrow_style(0);
        d.prev_arrow_style(0);
        assert_eq!(first_style(&d), start);
    }

    #[test]
    fn test_arrow_style_ignores_non_messages() {
        let mut d = SequenceDiagram::new();
        d.add_participant("A");
        d.add_note(NotePosition::Over, 0, 0, "n".to_string());
        assert!(d.next_arrow_style(0).is_none());
        assert!(d.prev_arrow_style(5).is_none());
    }

    #[test]
    fn test_reverse_keeps_arrow_style() {
        let mut d = SequenceDiagram::from_mermaid("sequenceDiagram\nA-->>B: Hi").unwrap();
        d.point_event_left(0);
        assert_eq!(first_style(&d), ArrowStyle::DashedArrow);
    }

    #[test]
    fn test_add_message_default_style() {
        let mut d = SequenceDiagram::new();
        d.add_participant("A");
        d.add_participant("B");
        d.add_message(0, 1, "Hi".to_string());
        assert_eq!(first_style(&d), ArrowStyle::SolidArrow);
    }

    #[test]
    fn test_parse_alias_and_actor() {
        let d = SequenceDiagram::from_mermaid(
            "sequenceDiagram\nparticipant A as Alice\nactor U as User\nactor Bob\nparticipant Carol\nU->>A: Hi",
        )
        .unwrap();

        assert_eq!(ids(&d), vec!["A", "U", "Bob", "Carol"]);
        assert_eq!(d.participants[0].label.as_deref(), Some("Alice"));
        assert!(!d.participants[0].actor);
        assert_eq!(d.participants[1].label.as_deref(), Some("User"));
        assert!(d.participants[1].actor);
        assert!(d.participants[2].actor);
        assert_eq!(d.participants[2].label, None);
        assert!(matches!(d.events[0], Event::Message { from: 1, to: 0, .. }));
    }

    #[test]
    fn test_exact_roundtrip_with_alias_actor_and_box() {
        let input = "sequenceDiagram\n    participant A as Alice\n    box Blue Team\n        actor U as User\n        participant B\n    end\n    U->>A: Hi\n    Note over A,B: Hey\n";
        let d = SequenceDiagram::from_mermaid(input).unwrap();

        assert_eq!(d.to_mermaid(), input);
    }

    #[test]
    fn test_implicit_participant_export() {
        let d = SequenceDiagram::from_mermaid("sequenceDiagram\nA->>B: x").unwrap();

        assert_eq!(
            d.to_mermaid(),
            "sequenceDiagram\n    participant A\n    participant B\n    A->>B: x\n"
        );
    }

    #[test]
    fn test_declaration_after_use_updates_entry() {
        let d =
            SequenceDiagram::from_mermaid("sequenceDiagram\nA->>B: x\nactor A as Alice").unwrap();

        assert_eq!(ids(&d), vec!["A", "B"]);
        assert!(d.participants[0].actor);
        assert_eq!(d.participants[0].label.as_deref(), Some("Alice"));
    }

    #[test]
    fn test_unreadable_ids_rejected() {
        for decl in [
            "participant re-xport",
            "participant a,b",
            "participant a:b",
            "actor ->> B: hi",
            "participant ->> B: hi",
        ] {
            let err = parse_err(&format!("sequenceDiagram\n{decl}"));
            assert!(err.starts_with("line 2:"), "{decl}: {err}");
        }
    }

    #[test]
    fn test_box_declaration_of_known_id_rejected() {
        for input in [
            "sequenceDiagram\nparticipant A\nbox\nparticipant A\nend",
            "sequenceDiagram\nA->>B: x\nbox\nparticipant B\nend",
        ] {
            let err = parse_err(input);
            assert!(
                err.contains("box participants must be declared before use"),
                "{err}"
            );
        }
    }

    #[test]
    fn test_empty_box_rejected() {
        let err = parse_err("sequenceDiagram\nparticipant A\nbox\nend");
        assert_eq!(err, "line 4: box has no participants");
    }

    #[test]
    fn test_reserved_word_ids_get_label() {
        let mut d = SequenceDiagram::new();
        d.add_participant("end");
        d.add_participant("Box");
        d.rename_participant(0, "loop");

        assert_eq!(ids(&d), vec!["end2", "Box2"]);
        assert_eq!(d.participants[0].label.as_deref(), Some("loop"));
        assert!(SequenceDiagram::from_mermaid(&d.to_mermaid()).is_ok());
    }

    #[test]
    fn test_empty_alias_rejected() {
        for decl in ["participant A as ", "actor  as Alice", "actor A as"] {
            let err = parse_err(&format!("sequenceDiagram\n{decl}"));
            assert!(
                err.starts_with("line 2: invalid participant declaration"),
                "{decl}: {err}"
            );
        }
    }

    #[test]
    fn test_add_participant_generates_ids() {
        let mut d = SequenceDiagram::new();
        d.add_participant("Alice Smith");
        d.add_participant("Bob");
        d.add_participant("Bob");
        d.add_participant(":::");

        assert_eq!(ids(&d), vec!["AliceSmith", "Bob", "Bob2", "P"]);
        assert_eq!(d.participants[0].label.as_deref(), Some("Alice Smith"));
        assert_eq!(d.participants[1].label, None);
        assert_eq!(d.participants[2].label.as_deref(), Some("Bob"));

        let parsed = SequenceDiagram::from_mermaid(&d.to_mermaid()).unwrap();
        assert_eq!(parsed.participants, d.participants);
    }

    #[test]
    fn test_rename_participant_rules() {
        let mut d = SequenceDiagram::new();
        d.add_participant("Alice");
        d.add_participant("Bob");

        d.rename_participant(0, "Carol");
        assert_eq!(d.participants[0], Participant::new("Carol"));

        d.rename_participant(0, "Bob");
        assert_eq!(d.participants[0].id, "Carol");
        assert_eq!(d.participants[0].label.as_deref(), Some("Bob"));

        d.rename_participant(0, "Carol");
        assert_eq!(d.participants[0], Participant::new("Carol"));

        d.rename_participant(1, "Big Bob");
        assert_eq!(d.participants[1].id, "Bob");
        assert_eq!(d.participants[1].label.as_deref(), Some("Big Bob"));

        d.rename_participant(1, "Other");
        assert_eq!(d.participants[1].id, "Bob");
        assert_eq!(d.participants[1].label.as_deref(), Some("Other"));
    }

    #[test]
    fn test_toggle_actor() {
        let mut d = SequenceDiagram::new();
        d.add_participant("Alice");

        assert_eq!(d.toggle_actor(0), Some(true));
        assert!(d.to_mermaid().contains("actor Alice"));
        assert_eq!(d.toggle_actor(0), Some(false));
        assert_eq!(d.toggle_actor(5), None);
    }

    #[test]
    fn test_swap_and_remove_keep_label_and_actor() {
        let mut d = SequenceDiagram::from_mermaid(
            "sequenceDiagram\nactor A as Alice\nparticipant B\nparticipant C as Carol",
        )
        .unwrap();

        d.swap_participants(0, 1);
        assert_eq!(ids(&d), vec!["B", "A", "C"]);
        assert!(d.participants[1].actor);
        assert_eq!(d.participants[1].label.as_deref(), Some("Alice"));

        d.remove_participant(0);
        assert_eq!(ids(&d), vec!["A", "C"]);
        assert!(d.participants[0].actor);
        assert_eq!(d.participants[1].label.as_deref(), Some("Carol"));
    }

    #[test]
    fn test_unsupported_keywords_rejected() {
        for kw in [
            "alt", "else", "opt", "par", "and", "critical", "option", "break", "rect",
        ] {
            let err = parse_err(&format!("sequenceDiagram\n{kw} x"));
            assert!(err.contains(&format!("'{kw}' blocks")), "{err}");
        }
        for kw in ["activate", "deactivate", "autonumber", "create", "destroy"] {
            let err = parse_err(&format!("sequenceDiagram\n{kw} A"));
            assert!(
                err.contains(&format!("'{kw}' is not supported yet")),
                "{err}"
            );
        }
        let err = parse_err("sequenceDiagram\nA->>B: x\nloop Every minute");
        assert_eq!(err, "line 3: 'loop' blocks are not supported yet");
    }

    #[test]
    fn test_activation_shorthand_rejected() {
        for to in ["+B", "-B"] {
            let err = parse_err(&format!("sequenceDiagram\nA->>{to}: x"));
            assert!(err.contains("activation shorthand"), "{err}");
        }
    }

    #[test]
    fn test_arrow_text_in_message_body() {
        let d = SequenceDiagram::from_mermaid("sequenceDiagram\nA->>B: a -> b").unwrap();
        match &d.events[0] {
            Event::Message { text, .. } => assert_eq!(text, "a -> b"),
            Event::Note { .. } => panic!("Expected Message event"),
        }
    }

    #[test]
    fn test_dashed_arrow_text_in_message_body() {
        let d = SequenceDiagram::from_mermaid("sequenceDiagram\nA-->>B: a -> b").unwrap();
        match &d.events[0] {
            Event::Message { text, style, .. } => {
                assert_eq!(text, "a -> b");
                assert_eq!(*style, ArrowStyle::DashedArrow);
            }
            Event::Note { .. } => panic!("Expected Message event"),
        }
    }

    #[test]
    fn test_bidirectional_arrow_rejected() {
        for arrow in ["<<->>", "<<-->>"] {
            let err = parse_err(&format!("sequenceDiagram\nA{arrow}B: x"));
            assert_eq!(
                err,
                "line 2: bidirectional arrows ('<<->>') are not supported yet"
            );
        }
    }

    #[test]
    fn test_participant_name_containing_arrow_prefix() {
        let err = parse_err("sequenceDiagram\ndata-xfer->>B: x");
        assert!(err.starts_with("line 2:"), "{err}");
    }

    #[test]
    fn test_unreadable_message_ids_rejected() {
        for msg in ["A->>b,c: x", "A->>B-xC: x", "a,b->>B: x"] {
            let err = parse_err(&format!("sequenceDiagram\n{msg}"));
            assert!(err.starts_with("line 2:"), "{msg}: {err}");
        }
    }

    #[test]
    fn test_hyphenated_id_in_message_parses() {
        let d = SequenceDiagram::from_mermaid("sequenceDiagram\nmy-svc->>B: x").unwrap();
        assert_eq!(ids(&d), vec!["my-svc", "B"]);
    }

    fn simulate_app_edits(d: &mut SequenceDiagram) {
        for i in 0..d.participants.len() {
            let mut style = ArrowStyle::default();
            for _ in 0..8 {
                d.events.push(Event::Message {
                    from: i,
                    to: 0,
                    text: "m".to_string(),
                    style,
                });
                style = style.next();
            }
            d.events.push(Event::Note {
                position: NotePosition::Over,
                participant_start: i,
                participant_end: 0,
                text: "n".to_string(),
            });
        }
    }

    #[test]
    fn test_unreadable_note_ids_rejected() {
        for line in [
            "Note over A as B: n",
            "A->>B as C: x",
            "Note over %%x: n",
            "Note over a-: n",
            "Note over participant x: n",
            "Note over loop x: n",
        ] {
            let err = parse_err(&format!("sequenceDiagram\n{line}"));
            assert!(err.starts_with("line 2:"), "{line}: {err}");
        }
    }

    #[test]
    fn test_spaced_and_keyword_like_ids_accepted() {
        let d = SequenceDiagram::from_mermaid(
            "sequenceDiagram\nparticipant Actor\nparticipant Alice Smith",
        )
        .unwrap();
        assert_eq!(ids(&d), vec!["Actor", "Alice Smith"]);
    }

    #[test]
    fn test_nested_box_rejected() {
        let err = parse_err("sequenceDiagram\nbox\nparticipant A\nbox\nend");
        assert_eq!(err, "line 4: nested boxes are not supported");
    }

    #[test]
    fn test_valid_inputs_reimport_equal() {
        let inputs = [
            "sequenceDiagram\nparticipant A as Alice\nactor U as User\nU->>A: Hi",
            "sequenceDiagram\nparticipant Alice Smith\nparticipant my-svc\nAlice Smith->>my-svc: x",
            "sequenceDiagram\nbox Green Team\nactor A\nparticipant my-svc as Svc\nend\nA->>my-svc: x\nNote over A,my-svc: n\nNote left of A: l",
            "sequenceDiagram\nA->>B: 1\nA-->>B: 2\nA->B: 3\nA-->B: 4\nA-xB: 5\nA--xB: 6\nA-)B: 7\nA--)B: 8",
        ];
        for input in inputs {
            let mut first = SequenceDiagram::from_mermaid(input).unwrap();
            simulate_app_edits(&mut first);
            let second = SequenceDiagram::from_mermaid(&first.to_mermaid()).unwrap();
            assert!(first == second, "{input}");
        }
    }

    #[test]
    fn test_hyphenated_participant_name() {
        let d = SequenceDiagram::from_mermaid("sequenceDiagram\nmy-svc->>B: x").unwrap();
        assert_eq!(ids(&d), vec!["my-svc", "B"]);
    }

    #[test]
    fn test_keyword_prefixed_participant_name() {
        let d = SequenceDiagram::from_mermaid("sequenceDiagram\nloop->>Bob: x").unwrap();
        assert_eq!(ids(&d), vec!["loop", "Bob"]);
    }

    #[test]
    fn test_roundtrip_with_comments() {
        let clean = "sequenceDiagram\nparticipant Alice\nparticipant Bob\nAlice->>Bob: Hi";
        let noisy =
            "sequenceDiagram\n%% c\nparticipant Alice\n\nparticipant Bob\n%% d\nAlice->>Bob: Hi";
        let exported = SequenceDiagram::from_mermaid(noisy).unwrap().to_mermaid();
        let expected = SequenceDiagram::from_mermaid(clean).unwrap().to_mermaid();
        assert_eq!(exported, expected);
    }

    #[test]
    fn test_unclosed_box_rejected() {
        let err = parse_err("sequenceDiagram\nbox Blue Group\nparticipant A");
        assert!(err.contains("never closed"));
    }

    #[test]
    fn test_from_mermaid_basic() {
        let input = "sequenceDiagram
    participant Alice
    participant Bob
    Alice->>Bob: Hello
    Bob->>Alice: Hi";
        let diagram = SequenceDiagram::from_mermaid(input).unwrap();
        assert_eq!(ids(&diagram), vec!["Alice", "Bob"]);
        assert_eq!(diagram.events.len(), 2);
    }

    #[test]
    fn test_from_mermaid_auto_participants() {
        let input = "sequenceDiagram
    Alice->>Bob: Hello";
        let diagram = SequenceDiagram::from_mermaid(input).unwrap();
        assert_eq!(ids(&diagram), vec!["Alice", "Bob"]);
    }

    #[test]
    fn test_invalid_header() {
        assert_eq!(
            parse_err("Alice->>Bob: Hello"),
            "line 1: first line must be 'sequenceDiagram'"
        );
        assert_eq!(
            parse_err("%% only a comment\n"),
            "missing 'sequenceDiagram' header"
        );
    }

    #[test]
    fn test_note_right() {
        let input = "sequenceDiagram
    participant Alice
    Note right of Alice: This is a note";
        let diagram = SequenceDiagram::from_mermaid(input).unwrap();
        assert_eq!(diagram.events.len(), 1);
        if let Event::Note {
            position,
            participant_start,
            text,
            ..
        } = &diagram.events[0]
        {
            assert_eq!(*position, NotePosition::Right);
            assert_eq!(*participant_start, 0);
            assert_eq!(text, "This is a note");
        } else {
            panic!("Expected Note event");
        }
    }

    #[test]
    fn test_note_over_multiple() {
        let input = "sequenceDiagram
    participant Alice
    participant Bob
    Note over Alice,Bob: Spanning note";
        let diagram = SequenceDiagram::from_mermaid(input).unwrap();
        assert_eq!(diagram.events.len(), 1);
        if let Event::Note {
            position,
            participant_start,
            participant_end,
            text,
        } = &diagram.events[0]
        {
            assert_eq!(*position, NotePosition::Over);
            assert_eq!(*participant_start, 0);
            assert_eq!(*participant_end, 1);
            assert_eq!(text, "Spanning note");
        } else {
            panic!("Expected Note event");
        }
    }

    #[test]
    fn test_note_roundtrip() {
        let mut diagram = SequenceDiagram::new();
        diagram.add_participant("Alice");
        diagram.add_participant("Bob");
        diagram.add_note(NotePosition::Right, 0, 0, "Right note".to_string());
        diagram.add_note(NotePosition::Left, 1, 1, "Left note".to_string());
        diagram.add_note(NotePosition::Over, 0, 1, "Over note".to_string());

        let mermaid = diagram.to_mermaid();
        let parsed = SequenceDiagram::from_mermaid(&mermaid).unwrap();

        assert_eq!(parsed.participants, diagram.participants);
        assert_eq!(parsed.events.len(), 3);
    }
}
