use super::models::{Event, NotePosition};
use anyhow::{Result, bail};

#[derive(Default, Clone)]
pub struct SequenceDiagram {
    pub participants: Vec<String>,
    pub events: Vec<Event>,
}

impl SequenceDiagram {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_participant(&mut self, name: String) {
        self.participants.push(name);
    }

    pub fn add_message(&mut self, from: usize, to: usize, text: String) {
        if from < self.participants.len() && to < self.participants.len() {
            self.events.push(Event::Message { from, to, text });
        }
    }

    pub fn insert_message(&mut self, after_index: usize, from: usize, to: usize, text: String) {
        if from < self.participants.len() && to < self.participants.len() {
            let insert_at = (after_index + 1).min(self.events.len());
            self.events
                .insert(insert_at, Event::Message { from, to, text });
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
    }

    pub fn to_mermaid(&self) -> String {
        let mut lines = vec!["sequenceDiagram".to_string()];

        for name in &self.participants {
            lines.push(format!("    participant {name}"));
        }

        for event in &self.events {
            match event {
                Event::Message { from, to, text } => {
                    if let (Some(from_name), Some(to_name)) =
                        (self.participants.get(*from), self.participants.get(*to))
                    {
                        lines.push(format!("    {from_name}->>{to_name}: {text}"));
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
                        if let (Some(start_name), Some(end_name)) = (
                            self.participants.get(*participant_start),
                            self.participants.get(*participant_end),
                        ) {
                            lines.push(format!(
                                "    Note {pos_str} {start_name},{end_name}: {text}"
                            ));
                        }
                    } else if let Some(name) = self.participants.get(*participant_start) {
                        lines.push(format!("    Note {pos_str} {name}: {text}"));
                    }
                }
            }
        }

        lines.join("\n") + "\n"
    }

    pub fn from_mermaid(input: &str) -> Result<Self> {
        let mut diagram = SequenceDiagram::new();
        let mut lines = input.lines();

        let first_line = lines.next().map_or("", str::trim);
        if first_line != "sequenceDiagram" {
            bail!("First line must be 'sequenceDiagram'");
        }

        for line in lines {
            let trimmed = line.trim();

            if trimmed.is_empty() {
                bail!("Empty lines are not supported");
            }
            if trimmed.starts_with("%%") {
                bail!("Comments are not supported");
            }

            // Parse participant
            if let Some(rest) = trimmed.strip_prefix("participant ") {
                let name = rest.trim().to_string();
                if name.is_empty() {
                    bail!("Invalid participant declaration: {line}");
                }
                if !diagram.participants.contains(&name) {
                    diagram.participants.push(name);
                }
                continue;
            }

            if let Some(rest) = trimmed.strip_prefix("Note ") {
                let (position, after_pos) = if let Some(after) = rest.strip_prefix("right of ") {
                    (NotePosition::Right, after)
                } else if let Some(after) = rest.strip_prefix("left of ") {
                    (NotePosition::Left, after)
                } else if let Some(after) = rest.strip_prefix("over ") {
                    (NotePosition::Over, after)
                } else {
                    bail!("Invalid note position: {line}");
                };

                let Some(colon_pos) = after_pos.find(':') else {
                    bail!("Invalid note syntax (missing ':'): {line}");
                };

                let participants_str = after_pos[..colon_pos].trim();
                let text = after_pos[colon_pos + 1..].trim().to_string();

                if position == NotePosition::Over && participants_str.contains(',') {
                    let parts: Vec<&str> = participants_str.split(',').map(str::trim).collect();
                    if parts.len() != 2 {
                        bail!("Note over must have exactly 2 participants: {line}");
                    }
                    let start_name = parts[0];
                    let end_name = parts[1];

                    if !diagram.participants.contains(&start_name.to_string()) {
                        diagram.participants.push(start_name.to_string());
                    }
                    if !diagram.participants.contains(&end_name.to_string()) {
                        diagram.participants.push(end_name.to_string());
                    }

                    let start_idx = diagram
                        .participants
                        .iter()
                        .position(|p| p == start_name)
                        .unwrap();
                    let end_idx = diagram
                        .participants
                        .iter()
                        .position(|p| p == end_name)
                        .unwrap();

                    diagram.events.push(Event::Note {
                        position,
                        participant_start: start_idx,
                        participant_end: end_idx,
                        text,
                    });
                } else {
                    let name = participants_str;
                    if name.is_empty() {
                        bail!("Invalid note syntax: {line}");
                    }

                    if !diagram.participants.contains(&name.to_string()) {
                        diagram.participants.push(name.to_string());
                    }

                    let idx = diagram.participants.iter().position(|p| p == name).unwrap();

                    diagram.events.push(Event::Note {
                        position,
                        participant_start: idx,
                        participant_end: idx,
                        text,
                    });
                }
                continue;
            }

            // Parse message
            if let Some(arrow_pos) = trimmed.find("->>") {
                let from_name = trimmed[..arrow_pos].trim();
                let rest = &trimmed[arrow_pos + 3..];

                let (to_name, message) = if let Some(colon_pos) = rest.find(':') {
                    let to = rest[..colon_pos].trim();
                    let msg = rest[colon_pos + 1..].trim();
                    (to, msg.to_string())
                } else {
                    bail!("Invalid message syntax (missing ':'): {line}");
                };

                if from_name.is_empty() || to_name.is_empty() {
                    bail!("Invalid message syntax: {line}");
                }

                if !diagram.participants.contains(&from_name.to_string()) {
                    diagram.participants.push(from_name.to_string());
                }
                if !diagram.participants.contains(&to_name.to_string()) {
                    diagram.participants.push(to_name.to_string());
                }

                let from_idx = diagram
                    .participants
                    .iter()
                    .position(|p| p == from_name)
                    .unwrap();

                let to_idx = diagram
                    .participants
                    .iter()
                    .position(|p| p == to_name)
                    .unwrap();

                diagram.events.push(Event::Message {
                    from: from_idx,
                    to: to_idx,
                    text: message,
                });
                continue;
            }

            bail!("Unsupported mermaid feature: {trimmed}");
        }

        Ok(diagram)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        let mut diagram = SequenceDiagram::new();
        diagram.add_participant("Alice".to_string());
        diagram.add_participant("Bob".to_string());
        diagram.add_message(0, 1, "Hello".to_string());
        diagram.add_message(1, 0, "Hi there".to_string());

        let mermaid = diagram.to_mermaid();
        let parsed = SequenceDiagram::from_mermaid(&mermaid).unwrap();

        assert_eq!(parsed.participants, diagram.participants);
        assert_eq!(parsed.events.len(), diagram.events.len());
    }

    #[test]
    fn test_from_mermaid_basic() {
        let input = "sequenceDiagram
    participant Alice
    participant Bob
    Alice->>Bob: Hello
    Bob->>Alice: Hi";
        let diagram = SequenceDiagram::from_mermaid(input).unwrap();
        assert_eq!(diagram.participants, vec!["Alice", "Bob"]);
        assert_eq!(diagram.events.len(), 2);
    }

    #[test]
    fn test_from_mermaid_auto_participants() {
        let input = "sequenceDiagram
    Alice->>Bob: Hello";
        let diagram = SequenceDiagram::from_mermaid(input).unwrap();
        assert_eq!(diagram.participants, vec!["Alice", "Bob"]);
    }

    #[test]
    fn test_invalid_header() {
        let input = "Alice->>Bob: Hello";
        let result = SequenceDiagram::from_mermaid(input);
        assert!(result.is_err());
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
        diagram.add_participant("Alice".to_string());
        diagram.add_participant("Bob".to_string());
        diagram.add_note(NotePosition::Right, 0, 0, "Right note".to_string());
        diagram.add_note(NotePosition::Left, 1, 1, "Left note".to_string());
        diagram.add_note(NotePosition::Over, 0, 1, "Over note".to_string());

        let mermaid = diagram.to_mermaid();
        let parsed = SequenceDiagram::from_mermaid(&mermaid).unwrap();

        assert_eq!(parsed.participants, diagram.participants);
        assert_eq!(parsed.events.len(), 3);
    }
}
